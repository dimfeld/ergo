pub mod dequeue;
pub mod handlers;
pub mod queue;

use crate::{database::PostgresPool, error::Error};
use serde::{Deserialize, Serialize};
use sqlx::Connection;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct InputCategory {
    pub input_category_id: i64,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
    pub input_id: i64,
    pub input_category_id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub payload_schema: serde_json::Value, // TODO make this a JsonSchema
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InputsLog {
    pub inputs_log_id: i64,
    pub input_id: i64,
    pub status: InputStatus,
    pub payload: serde_json::Value,
    pub error: serde_json::Value,
    pub time: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "input_status", rename_all = "lowercase")]
pub enum InputStatus {
    Pending,
    Success,
    Error,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InputInvocation {
    pub task_id: i64,
    pub task_trigger_id: i64,
    pub input_id: i64,
    pub inputs_log_id: uuid::Uuid,
    pub payload: serde_json::Value,
}

pub fn validate_input_payload(
    _input_id: i64,
    payload_schema: &serde_json::Value,
    payload: &serde_json::Value,
) -> Result<(), Error> {
    let compiled_schema = jsonschema::JSONSchema::compile(&payload_schema)?;
    compiled_schema.validate(&payload)?;
    Ok(())
}

pub async fn enqueue_input(
    pg: &PostgresPool,
    task_id: i64,
    input_id: i64,
    task_trigger_id: i64,
    task_trigger_local_id: String,
    payload_schema: &serde_json::Value,
    payload: serde_json::Value,
) -> Result<Uuid, Error> {
    validate_input_payload(input_id, payload_schema, &payload)?;

    let input_arrival_id = Uuid::new_v4();

    let mut conn = pg.acquire().await?;
    conn.transaction(|tx| {
        Box::pin(async move {
            sqlx::query!(
                r##"INSERT INTO event_queue
        (task_id, input_id, task_trigger_id, inputs_log_id, payload) VALUES
        ($1, $2, $3, $4, $5)"##,
                task_id,
                input_id,
                task_trigger_id,
                &input_arrival_id,
                payload
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                r##"INSERT INTO inputs_log
        (inputs_log_id, task_trigger_id, task_id, task_trigger_local_id, status, payload)
        VALUES
        ($1, $2, $3, $4, 'pending', $5)"##,
                input_arrival_id,
                task_trigger_id,
                task_id,
                task_trigger_local_id,
                payload
            )
            .execute(&mut *tx)
            .await?;

            Ok::<(), Error>(())
        })
    })
    .await?;

    Ok(input_arrival_id)
}
