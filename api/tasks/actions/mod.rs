pub mod accounts;
pub mod dequeue;
pub mod execute;
pub mod handlers;
pub mod queue;
pub mod template;

mod http_executor;
mod js_executor;
mod raw_command_executor;

use ergo_database::object_id::{AccountId, ActionCategoryId, ActionId, TaskId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use self::template::TemplateFields;

#[derive(Debug, Serialize, Deserialize)]
pub struct ActionCategory {
    pub action_category_id: ActionCategoryId,
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
    pub action_id: ActionId,
    pub action_category_id: ActionCategoryId,
    pub name: String,
    pub description: Option<String>,
    pub executor_id: String,
    pub executor_template: serde_json::Map<String, serde_json::Value>,
    pub template_fields: TemplateFields,
    pub account_required: bool,
    /// A synchronous script to postprocess the executor's result. The value returned from the
    /// script will replace the result, or the script can throw an error to mark the whole
    /// action as a failure.
    pub postprocess_script: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskAction {
    pub action_id: ActionId,
    pub task_local_id: String,
    pub task_id: TaskId,
    pub account_id: Option<AccountId>,
    pub name: String,
    pub action_template: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Eq, sqlx::Type)]
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
    pub task_id: TaskId,
    pub task_action_local_id: String,
    pub actions_log_id: Uuid,
    pub input_arrival_id: Option<Uuid>,
    pub payload: serde_json::Value,
}
