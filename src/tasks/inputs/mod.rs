pub mod dequeue;
pub mod handlers;
pub mod queue;

use crate::{
    database::PostgresPool,
    error::Error,
    notifications::{Notification, NotifyEvent},
};
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

pub struct EnqueueInputOptions<'a> {
    pub pg: &'a PostgresPool,
    pub notifications: Option<crate::notifications::NotificationManager>,
    pub org_id: Uuid,
    pub task_id: i64,
    pub task_name: String,
    pub input_id: i64,
    pub task_trigger_id: i64,
    pub task_trigger_local_id: String,
    pub task_trigger_name: String,
    pub payload_schema: &'a serde_json::Value,
    pub payload: serde_json::Value,
}

pub async fn enqueue_input(options: EnqueueInputOptions<'_>) -> Result<Uuid, Error> {
    let EnqueueInputOptions {
        pg,
        notifications,
        org_id,
        task_id,
        task_name,
        input_id,
        task_trigger_id,
        task_trigger_local_id,
        task_trigger_name,
        payload_schema,
        payload,
    } = options;

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
                &input_arrival_id,
                task_trigger_id,
                task_id,
                &task_trigger_local_id,
                &payload
            )
            .execute(&mut *tx)
            .await?;

            if let Some(notify) = &notifications {
                let notification = Notification {
                    task_id,
                    local_id: task_trigger_local_id,
                    local_object_id: Some(task_trigger_id),
                    local_object_name: task_trigger_name,
                    error: None,
                    event: NotifyEvent::InputArrived,
                    task_name,
                    log_id: Some(input_arrival_id),
                    payload: Some(payload),
                };
                notify.notify(&mut *tx, &org_id, notification).await?;
            }

            Ok::<(), Error>(())
        })
    })
    .await?;

    Ok(input_arrival_id)
}
