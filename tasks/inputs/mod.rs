#[cfg(not(target_family = "wasm"))]
pub mod dequeue;
#[cfg(not(target_family = "wasm"))]
pub mod queue;

#[cfg(not(target_family = "wasm"))]
pub use queue::{enqueue_input, EnqueueInputOptions};

use crate::error::Error;
use ergo_database::object_id::{InputCategoryId, InputId, TaskId, TaskTriggerId, UserId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(not(target_family = "wasm"), derive(sqlx::Type))]
#[cfg_attr(
    not(target_family = "wasm"),
    sqlx(type_name = "input_status", rename_all = "lowercase")
)]
#[serde(rename_all = "lowercase")]
pub enum InputStatus {
    Pending,
    Success,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InputInvocation {
    pub task_id: TaskId,
    pub task_trigger_id: TaskTriggerId,
    pub input_id: InputId,
    pub inputs_log_id: uuid::Uuid,
    pub payload: serde_json::Value,
    pub user_id: UserId,
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
