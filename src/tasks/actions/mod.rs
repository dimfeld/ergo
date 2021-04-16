pub mod queue_drain;

use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ActionCategory {
    pub action_category_id: i64,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ActionExecutor {
    #[serde(rename = "http")]
    Http,
    #[serde(rename = "nomad")]
    Nomad,
    #[serde(rename = "input")]
    Input,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Action {
    pub action_id: i64,
    pub action_category_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Box<serde_json::value::RawValue>,
    pub executor: ActionExecutor,
    pub executor_data: Box<serde_json::value::RawValue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ActionStatus {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "error")]
    Error,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ActionInvocation {
    pub task_id: i64,
    pub task_trigger_id: Option<i64>,
    pub action_id: i64,
    pub payload: serde_json::Value,
}
