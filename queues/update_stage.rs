use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgConnection;

use crate::{generic_stage::NOTIFY_CHANNEL, Error};

#[derive(Debug, Clone, Default)]
pub struct JobUpdate<T: Serialize + Send + Sync> {
    pub payload: Option<T>,
    pub run_at: Option<DateTime<Utc>>,
}

pub async fn remove_pending_job(
    tx: &mut PgConnection,
    queue: &str,
    job_id: &str,
) -> Result<(), Error> {
    sqlx::query!(
        "INSERT INTO queue_stage (queue, job_id, operation)
        VALUES ($1, $2, 'remove')",
        queue,
        job_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query(format!(r##"NOTIFY "{}""##, NOTIFY_CHANNEL).as_str())
        .execute(tx)
        .await?;
    Ok(())
}

pub async fn update_pending_job<T: Serialize + Send + Sync>(
    tx: &mut PgConnection,
    queue: &str,
    job_id: &str,
    alteration: &JobUpdate<T>,
) -> Result<(), Error> {
    sqlx::query!(
        "INSERT INTO queue_stage (queue, job_id, payload, run_at, operation)
        VALUES ($1, $2, $3, $4, 'update')",
        queue,
        job_id,
        sqlx::types::Json(alteration.payload.as_ref()) as _,
        alteration.run_at,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query(format!(r##"NOTIFY "{}""##, NOTIFY_CHANNEL).as_str())
        .execute(tx)
        .await?;
    Ok(())
}
