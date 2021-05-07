use crate::error::{Error, Result};
use sqlx::Executor;
use std::future::Future;

pub async fn new_object_id(
    tx: &'_ mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<i64, sqlx::Error> {
    let id = sqlx::query_scalar!(
        "INSERT INTO object_ids (object_id) VALUES (DEFAULT) RETURNING object_id"
    )
    .fetch_one(&mut *tx)
    .await?;
    Ok(id)
}

pub async fn new_object_id_with_value(
    tx: &'_ mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: Option<&i64>,
) -> Result<i64, sqlx::Error> {
    if let Some(id) = id {
        sqlx::query_scalar!("INSERT INTO object_ids (object_id) VALUES ($1)", id)
            .execute(&mut *tx)
            .await?;
        Ok(*id)
    } else {
        new_object_id(tx).await
    }
}
