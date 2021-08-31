pub mod dequeue;
pub mod handlers;
pub mod queue;

use crate::{
    error::Error,
    notifications::{Notification, NotifyEvent},
};
use ergo_database::{
    object_id::{InputCategoryId, InputId, OrgId, TaskId, TaskTriggerId},
    PostgresPool,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::Connection;
use uuid::Uuid;

use super::Task;

#[derive(Debug, Serialize, Deserialize)]
pub struct InputCategory {
    pub input_category_id: InputCategoryId,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize, JsonSchema, Deserialize, PartialEq, Eq)]
pub struct Input {
    pub input_id: InputId,
    pub input_category_id: Option<InputCategoryId>,
    pub name: String,
    pub description: Option<String>,
    pub payload_schema: serde_json::Value, // TODO make this a JsonSchema
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InputsLog {
    pub inputs_log_id: i64,
    pub input_id: InputId,
    pub status: InputStatus,
    pub payload: serde_json::Value,
    pub error: serde_json::Value,
    pub time: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, JsonSchema, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "input_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum InputStatus {
    Pending,
    Success,
    Error,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InputInvocation {
    pub task_id: TaskId,
    pub task_trigger_id: TaskTriggerId,
    pub input_id: InputId,
    pub inputs_log_id: uuid::Uuid,
    pub payload: serde_json::Value,
}

pub fn validate_input_payload(
    _input_id: &InputId,
    payload_schema: &serde_json::Value,
    payload: &serde_json::Value,
) -> Result<(), Error> {
    let compiled_schema = jsonschema::JSONSchema::compile(&payload_schema)?;
    compiled_schema.validate(&payload)?;
    Ok(())
}

pub struct EnqueueInputOptions<'a> {
    /// If true, apply the input immediately instead of adding it to the queue.
    pub run_immediately: bool,
    /// If both `run_immediately` and `immediate_actions` are true, then run actions immediately
    /// instead of enqueueing them.
    pub immediate_actions: bool,
    pub pg: &'a PostgresPool,
    pub notifications: Option<crate::notifications::NotificationManager>,
    pub org_id: OrgId,
    pub task_id: TaskId,
    pub task_name: String,
    pub input_id: InputId,
    pub task_trigger_id: TaskTriggerId,
    pub task_trigger_local_id: String,
    pub task_trigger_name: String,
    pub payload_schema: &'a serde_json::Value,
    pub payload: serde_json::Value,
}

pub async fn enqueue_input(options: EnqueueInputOptions<'_>) -> Result<Uuid, Error> {
    let EnqueueInputOptions {
        run_immediately,
        immediate_actions,
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

    validate_input_payload(&input_id, payload_schema, &payload)?;

    let input_arrival_id = Uuid::new_v4();

    let immediate_data = if run_immediately {
        Some((notifications.clone(), payload.clone()))
    } else {
        None
    };

    let mut conn = pg.acquire().await?;
    conn.transaction(|tx| {
        let input_id = input_id.clone();
        let task_id = task_id.clone();
        let task_trigger_id = task_trigger_id.clone();
        Box::pin(async move {
            if !run_immediately {
                sqlx::query!(
                    r##"INSERT INTO event_queue
            (task_id, input_id, task_trigger_id, inputs_log_id, payload) VALUES
            ($1, $2, $3, $4, $5)"##,
                    &task_id.0,
                    &input_id.0,
                    &task_trigger_id.0,
                    &input_arrival_id,
                    payload
                )
                .execute(&mut *tx)
                .await?;
            }

            sqlx::query!(
                r##"INSERT INTO inputs_log
        (inputs_log_id, task_trigger_id, task_id, task_trigger_local_id, status, payload)
        VALUES
        ($1, $2, $3, $4, 'pending', $5)"##,
                &input_arrival_id,
                &task_trigger_id.0,
                &task_id.0,
                &task_trigger_local_id,
                &payload
            )
            .execute(&mut *tx)
            .await?;

            if let Some(notify) = notifications {
                let notification = Notification {
                    task_id,
                    local_id: task_trigger_local_id,
                    local_object_id: Some(task_trigger_id.into_inner()),
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

    if let Some((notifications, payload)) = immediate_data {
        Task::apply_input(
            pg,
            notifications,
            task_id,
            input_id,
            task_trigger_id,
            input_arrival_id,
            payload,
            immediate_actions,
        )
        .await?;
    }

    Ok(input_arrival_id)
}
