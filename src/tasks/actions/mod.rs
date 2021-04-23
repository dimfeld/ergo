pub mod accounts;
pub mod execute;
pub mod queue;
pub mod template;

mod http_executor;
mod raw_command_executor;

use fxhash::FxHashMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use self::template::TemplateFields;

#[derive(Debug, Serialize, Deserialize)]
pub struct ActionCategory {
    pub action_category_id: i64,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Executor {
    pub executor_id: String,
    pub name: String,
    pub description: Option<String>,
    pub template_fields: TemplateFields,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Action {
    pub action_id: i64,
    pub action_category_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub executor_id: String,
    pub executor_template: serde_json::Map<String, serde_json::Value>,
    pub template_fields: TemplateFields,
    pub account_required: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskAction {
    pub task_action_id: i64,
    pub action_id: i64,
    pub task_id: i64,
    pub account_id: Option<i64>,
    pub name: String,
    pub action_template: Option<serde_json::Map<String, serde_json::Value>>,
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
    pub task_action_id: i64,
    pub payload: serde_json::Value,
}
