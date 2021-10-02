use super::postgres_drain::Drainer;
use crate::{error::Error, Job};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ergo_database::sql_insert_parameters;
use serde::Serialize;
use sqlx::{PgConnection, Postgres, Transaction};
use std::{borrow::Cow, time::Duration};

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
            .unwrap_or_else(|| Cow::Owned(uuid::Uuid::new_v4().to_string()))
    }

    pub async fn enqueue(self, tx: &mut PgConnection) -> Result<(), Error> {
        enqueue_jobs(tx, &[self]).await
    }
}

pub async fn enqueue_jobs<T: Serialize + Send + Sync>(
    tx: &mut PgConnection,
    jobs: &[QueueJob<'_, T>],
) -> Result<(), Error> {
    let q = format!(
        r##"INSERT INTO queue_stage (queue, job_id, payload, timeout, max_retries, run_at, retry_backoff)
            VALUES
            {}"##,
        sql_insert_parameters::<7>(jobs.len())
    );

    let mut query = sqlx::query(&q);
    for job in jobs {
        eprintln!("Enqueueing job to {}", job.queue);
        query = query
            .bind(job.queue)
            .bind(job.get_id_or_default().to_string())
            .bind(sqlx::types::Json(&job.payload))
            .bind(job.timeout.map(|t| t.as_millis() as i32))
            .bind(job.max_retries.map(|i| i as i32))
            .bind(job.run_at)
            .bind(job.retry_backoff.map(|i| i.as_millis() as i32));
    }

    query.execute(&mut *tx).await?;
    eprintln!("Enqueued jobs");

    Ok(())
}

pub struct QueueDrainer {}

#[async_trait]
impl Drainer for QueueDrainer {
    type Error = Error;

    fn lock_key(&self) -> i64 {
        80235523425
    }

    async fn get(
        &'_ self,
        tx: &mut Transaction<Postgres>,
    ) -> Result<Vec<(Cow<'static, str>, super::Job<'_>)>, Error> {
        let results = sqlx::query!(
            "SELECT id, queue, job_id, payload,
            timeout, max_retries, run_at, retry_backoff
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
                let payload = serde_json::to_vec(&row.payload)?;
                Ok((
                    Cow::Owned(row.queue),
                    Job {
                        id: row.job_id,
                        retry_backoff: row.retry_backoff.map(|r| Duration::from_millis(r as u64)),
                        run_at: row.run_at,
                        max_retries: row.max_retries.map(|r| r as u32),
                        timeout: row.timeout.map(|t| Duration::from_millis(t as u64)),
                        payload: Cow::Owned(payload),
                    },
                ))
            })
            .collect::<Result<Vec<_>, Error>>()
    }
}
