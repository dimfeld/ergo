#[cfg(not(target_family = "wasm"))]
pub mod accounts;
#[cfg(not(target_family = "wasm"))]
pub mod dequeue;
pub mod execute;
#[cfg(not(target_family = "wasm"))]
pub mod queue;
#[cfg(not(target_family = "wasm"))]
pub use queue::enqueue_actions;
pub mod template;

mod http_executor;
mod js_executor;
mod raw_command_executor;
mod send_input_executor;

#[cfg(target_family = "wasm")]
use anyhow::anyhow;
use ergo_database::object_id::{AccountId, ActionCategoryId, ActionId, TaskId, UserId};
use futures::future::ready;
use fxhash::FxHashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use uuid::Uuid;

use crate::{scripting, ActionValidateError, ActionValidateErrors};

use self::{
    execute::{ScriptOrTemplate, EXECUTOR_REGISTRY},
    template::TemplateFields,
};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[cfg_attr(not(target_family = "wasm"), derive(sqlx::FromRow))]
pub struct Action {
    pub action_id: ActionId,
    pub action_category_id: ActionCategoryId,
    pub name: String,
    pub description: Option<String>,
    pub executor_id: String,
    pub executor_template: ScriptOrTemplate,
    pub template_fields: TemplateFields,
    pub timeout: Option<i32>,
    /// A script that processes the executor's JSON result.
    /// The result is exposed in the variable `result` and the action's payload
    /// is exposed as `payload`. The value returned will replace the executor's
    /// return value, or an error can be thrown to mark the action as failed.
    pub postprocess_script: Option<String>,
    pub account_required: bool,
    #[serde(default)]
    pub account_types: Vec<String>,
}

impl Action {
    pub async fn validate(&self) -> Result<(), ActionValidateErrors> {
        let executor = EXECUTOR_REGISTRY
            .get(self.executor_id.as_str())
            .ok_or_else(|| ActionValidateError::UnknownExecutor(self.executor_id.clone()))?;

        let values_map = match &self.executor_template {
            ScriptOrTemplate::Template(values) => {
                values.iter().cloned().collect::<FxHashMap<_, _>>()
            }
            ScriptOrTemplate::Script(s) => run_script(s)
                .await
                .map_err(ActionValidateError::ScriptError)?,
        };

        self::template::validate(
            "action",
            Some(&self.action_id),
            executor.template_fields(),
            &values_map,
        )
        .map_err(ActionValidateError::TemplateError)?;
        Ok(())
    }
}

#[cfg(not(target_family = "wasm"))]
async fn run_script(s: &str) -> Result<FxHashMap<String, serde_json::Value>, anyhow::Error> {
    let s = s.to_string();
    scripting::POOL
        .run(move || {
            let mut runtime = scripting::create_simple_runtime();
            let values = runtime
                .run_expression::<FxHashMap<String, serde_json::Value>>("<action template>", &s);
            ready(values)
        })
        .await
}

#[cfg(target_family = "wasm")]
async fn run_script(s: &str) -> Result<FxHashMap<String, serde_json::Value>, anyhow::Error> {
    let js_value = js_sys::eval(s).map_err(|e| anyhow!("{:?}", e))?;
    serde_wasm_bindgen::from_value(js_value).map_err(|e| anyhow!("{:?}", e))
}

pub type TaskActionTemplate = Vec<(String, serde_json::Value)>;

#[derive(Debug, JsonSchema, Serialize, Deserialize)]
pub struct TaskAction {
    pub action_id: ActionId,
    pub task_local_id: String,
    pub task_id: TaskId,
    pub account_id: Option<AccountId>,
    pub name: String,
    pub action_template: Option<TaskActionTemplate>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(target_family = "wasm"), derive(sqlx::Type))]
#[cfg_attr(
    not(target_family = "wasm"),
    sqlx(type_name = "action_status", rename_all = "lowercase")
)]
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
    pub user_id: UserId,
    pub payload: serde_json::Value,
}

pub type ActionInvocations = SmallVec<[ActionInvocation; 1]>;

#[derive(Debug, Deserialize)]
pub struct TaskActionInvocation {
    pub name: String,
    pub payload: serde_json::Value,
}

pub type TaskActionInvocations = SmallVec<[TaskActionInvocation; 1]>;
