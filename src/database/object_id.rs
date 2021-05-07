use crate::error::{Error, Result};
use sqlx::Executor;
use std::future::Future;

pub async fn new_object_id(
    tx: &'_ mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<i64, sqlx::Error> {
    let task_id = sqlx::query_scalar!(
        "INSERT INTO object_ids (object_id) VALUES (DEFAULT) RETURNING object_id"
    )
    .fetch_one(&mut *tx)
    .await?;
    Ok(task_id)
}
