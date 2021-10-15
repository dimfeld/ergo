use super::postgres_drain::Drainer;
use crate::{
    error::Error,
    postgres_drain::{DrainResult, QueueOperation},
    Job,
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ergo_database::{new_uuid, sql_insert_parameters};
use serde::Serialize;
use smallvec::SmallVec;
use sqlx::{PgConnection, Postgres, Transaction};
use std::{borrow::Cow, str::FromStr, time::Duration};

pub struct QueueJob<'a, T: Serialize + Send + Sync> {
    pub queue: &'a str,
    pub id: Option<&'a str>,
    pub payload: &'a T,
    pub timeout: Option<Duration>,
    pub max_retries: Option<u32>,
    pub run_at: Option<DateTime<Utc>>,
    pub retry_backoff: Option<Duration>,
}

impl<'a, T: Serialize + Send + Sync> QueueJob<'a, T> {
    #[must_use]
    pub fn new(queue: &'a str, payload: &'a T) -> Self {
        QueueJob {
            queue,
            payload,
            id: None,
            timeout: None,
            max_retries: None,
            run_at: None,
            retry_backoff: None,
        }
    }

    #[must_use]
    pub fn id(&mut self, id: &'a str) -> &mut Self {
        self.id = Some(id);
        self
    }

    #[must_use]
    pub fn timeout(&mut self, timeout: Duration) -> &mut Self {
        self.timeout = Some(timeout);
        self
    }

    #[must_use]
    pub fn max_retries(&mut self, max_retries: u32) -> &mut Self {
        self.max_retries = Some(max_retries);
        self
    }

    #[must_use]
    pub fn run_at(&mut self, run_at: DateTime<Utc>) -> &mut Self {
        self.run_at = Some(run_at);
        self
    }

    #[must_use]
    pub fn retry_backoff(&mut self, retry_backoff: Duration) -> &mut Self {
        self.retry_backoff = Some(retry_backoff);
        self
    }

    fn get_id_or_default(&self) -> Cow<'a, str> {
        self.id
            .map(|s| Cow::Borrowed(s))
            .unwrap_or_else(|| Cow::Owned(new_uuid().to_string()))
    }

    /// Enqueue a job and return the job's ID
    pub async fn enqueue(self, tx: &mut PgConnection) -> Result<String, Error> {
        let result = enqueue_jobs(tx, &[self]).await?;
        Ok(result.into_iter().next().unwrap())
    }
}

pub async fn enqueue_jobs<T: Serialize + Send + Sync>(
    tx: &mut PgConnection,
    jobs: &[QueueJob<'_, T>],
) -> Result<SmallVec<[String; 1]>, Error> {
    #[derive(sqlx::FromRow)]
    struct Result {
        job_id: String,
    }

    let q = format!(
        r##"INSERT INTO queue_stage (queue, job_id, payload, timeout, max_retries, run_at, retry_backoff)
            VALUES
            {}
            RETURNING job_id"##,
        sql_insert_parameters::<7>(jobs.len())
    );

    let mut query = sqlx::query_as(&q);
    for job in jobs {
        query = query
            .bind(job.queue)
            .bind(job.get_id_or_default().to_string())
            .bind(sqlx::types::Json(&job.payload))
            .bind(job.timeout.map(|t| t.as_millis() as i32))
            .bind(job.max_retries.map(|i| i as i32))
            .bind(job.run_at)
            .bind(job.retry_backoff.map(|i| i.as_millis() as i32));
    }

    let ids: Vec<Result> = query.fetch_all(&mut *tx).await?;

    sqlx::query(format!(r##"NOTIFY "{}""##, NOTIFY_CHANNEL).as_str())
        .execute(tx)
        .await?;

    Ok(ids.into_iter().map(|r| r.job_id).collect())
}

pub struct QueueDrainer {}

const NOTIFY_CHANNEL: &'static str = "queue-generic";

#[async_trait]
impl Drainer for QueueDrainer {
    type Error = Error;

    fn notify_channel(&self) -> Option<String> {
        Some(NOTIFY_CHANNEL.to_string())
    }

    fn lock_key(&self) -> i64 {
        80235523425
    }

    async fn get(&'_ self, tx: &mut Transaction<Postgres>) -> Result<Vec<DrainResult<'_>>, Error> {
        let results = sqlx::query!(
            "SELECT id, queue, job_id, payload,
            timeout, max_retries, run_at, retry_backoff, operation
            FROM queue_stage
            ORDER BY id LIMIT 50"
        )
        .fetch_all(&mut *tx)
        .await?;

        if let Some(max_id) = results.last().map(|r| r.id) {
            sqlx::query!("DELETE FROM queue_stage WHERE id <= $1", max_id)
                .execute(&mut *tx)
                .await?;
        }

        results
            .into_iter()
            .map(|row| {
                let operation = row
                    .operation
                    .map(|op| QueueOperation::from_str(op.as_str()).unwrap()) // TODO no unwrap
                    .unwrap_or(QueueOperation::Add);

                let payload = match (&operation, row.payload.as_ref()) {
                    (QueueOperation::Update, None) => Cow::Borrowed("".as_bytes()),
                    (_, None) => Cow::Borrowed("null".as_bytes()),
                    (_, Some(v)) => Cow::Owned(serde_json::to_vec(v)?),
                };

                Ok(DrainResult {
                    queue: Cow::Owned(row.queue),
                    operation,
                    job: Job {
                        id: row.job_id,
                        retry_backoff: row.retry_backoff.map(|r| Duration::from_millis(r as u64)),
                        run_at: row.run_at,
                        max_retries: row.max_retries.map(|r| r as u32),
                        timeout: row.timeout.map(|t| Duration::from_millis(t as u64)),
                        payload,
                    },
                })
            })
            .collect::<Result<Vec<_>, Error>>()
    }
}
