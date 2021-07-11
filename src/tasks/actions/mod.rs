pub mod accounts;
pub mod dequeue;
pub mod execute;
pub mod handlers;
pub mod queue;
pub mod template;

mod http_executor;
mod raw_command_executor;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    pub action_id: i64,
    pub task_local_id: String,
    pub task_id: i64,
    pub account_id: Option<i64>,
    pub name: String,
    pub action_template: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, sqlx::Type)]
#[sqlx(type_name = "action_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ActionStatus {
    Success,
    Pending,
    Running,
    Error,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ActionInvocation {
    pub task_id: i64,
    pub task_action_local_id: String,
    pub actions_log_id: Uuid,
    pub input_arrival_id: Option<Uuid>,
    pub payload: serde_json::Value,
}
