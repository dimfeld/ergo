use crate::error::Error;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct InputCategory {
    pub input_category_id: i64,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
    pub input_id: i64,
    pub input_category_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub payload_schema: serde_json::Value, // TODO make this a JsonSchema
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InputsLog {
    pub inputs_log_id: i64,
    pub input_id: i64,
    pub payload: serde_json::Value,
    pub error: serde_json::Value,
    pub time: chrono::DateTime<chrono::Utc>,
}

pub async fn enqueue_input(
    task_id: i64,
    input_id: i64,
    task_trigger_id: i64,
    payload: Box<serde_json::value::RawValue>,
) -> Result<(), Error> {
    Ok(())
}
