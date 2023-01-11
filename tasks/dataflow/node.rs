use crate::{actions::TaskActionInvocation, Error, Result};
use fxhash::FxHashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

pub use js::ConsoleMessage;

#[derive(Debug)]
pub(crate) struct NodeResult {
    pub state: serde_json::Value,
    pub action: Option<TaskActionInvocation>,
    pub console: Vec<ConsoleMessage>,
}

impl NodeResult {
    pub fn empty() -> Self {
        Self {
            state: serde_json::Value::Null,
            action: None,
            console: Vec::new(),
        }
    }
}

impl From<(serde_json::Value, Vec<ConsoleMessage>)> for NodeResult {
    fn from((state, console): (serde_json::Value, Vec<ConsoleMessage>)) -> Self {
        Self {
            state,
            action: None,
            console,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DataFlowNode {
    pub name: String,
    /// If true, run the node even if some of its inputs are null.
    /// If false, do not run the node if any of its inputs are null.
    pub allow_null_inputs: bool,
    pub func: DataFlowNodeFunction,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DataFlowNodeFunction {
    /// This node functions to take a task trigger and pass its data to other nodes.
    Trigger(DataFlowTrigger),
    /// This node can trigger an action, controlled by the accompanying JavaScript code.
    Action(DataFlowAction),
    /// Plain Text
    Text(DataFlowText),
    /// A JavaScript expression or function body
    Js(DataFlowJs),
    // /// JavaScript code to be fed into another node
    //JsCode(DataFlowJsLibrary),
    Table,
    Graph,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DataFlowText {
    pub body: String,
    pub render_as: TextRenderAs,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TextRenderAs {
    PlainText,
    Markdown,
    Html,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DataFlowTrigger {
    /// The `task_trigger_local_id` of the trigger that this node should listen for.
    pub local_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "source", rename_all = "camelCase")]
pub enum DataFlowJsLibrary {
    Inline {
        body: String,
    },
    Npm {
        package: String,
        resolved: String,
        // TODO This is not ideal. Better to replace this with dynamic loading and caching instead of
        // storing the entire package in the DB.
        code: String,
    },
}

impl DataFlowJsLibrary {
    fn code(&self) -> &str {
        match self {
            Self::Inline { body } => body,
            Self::Npm { code, .. } => code,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum JsCodeFormat {
    Expression,
    Function,
    AsyncFunction,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DataFlowJs {
    pub code: String,
    pub format: JsCodeFormat,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DataFlowAction {
    pub action_id: String,
    pub payload_code: DataFlowJs,
}

pub(super) enum NodeInput {
    Single(serde_json::Value),
    Multiple(FxHashMap<String, serde_json::Value>),
}

impl From<NodeInput> for serde_json::Value {
    fn from(input: NodeInput) -> Self {
        match input {
            NodeInput::Single(value) => value,
            NodeInput::Multiple(map) => json!(map),
        }
    }
}

impl DataFlowNodeFunction {
    pub(super) async fn execute(
        &self,
        task_name: &str,
        node_name: &str,
        current_state: &serde_json::Value,
        input: NodeInput,
    ) -> Result<NodeResult> {
        match self {
            Self::Js(expr) => js::run_js(task_name, node_name, expr, current_state.clone(), input)
                .await
                .map(NodeResult::from),
            Self::Action(expr) => {
                evaluate_action_node(task_name, node_name, expr, current_state.clone(), input).await
            }
            Self::Trigger(_) => Ok(NodeResult {
                state: input.into(),
                action: None,
                console: Vec::new(),
            }),
            Self::Text(_) | Self::Table | Self::Graph => Ok(NodeResult::empty()),
        }
    }

    pub(super) fn persist_output(&self) -> bool {
        match self {
            Self::Js(_) | Self::Action(_) | Self::Trigger(_) => true,
            Self::Table | Self::Graph | Self::Text(_) => false,
        }
    }

    pub(super) fn output(&self, state: &serde_json::Value) -> serde_json::Value {
        match self {
            Self::Js(_) | Self::Action(_) | Self::Trigger(_) => state.clone(),
            Self::Text(node) => json!(node.body),
            // Self::JsCode(node) => json!(node.code()),
            Self::Table | Self::Graph => serde_json::Value::Null,
        }
    }
}

async fn evaluate_action_node(
    task_name: &str,
    node_name: &str,
    action: &DataFlowAction,
    current_state: serde_json::Value,
    input: NodeInput,
) -> Result<NodeResult> {
    let (result, console) = js::run_js(
        task_name,
        node_name,
        &action.payload_code,
        current_state,
        input,
    )
    .await?;

    let action_payload = match &result {
        serde_json::Value::Null => None,
        serde_json::Value::Object(_) => Some(result.clone()),
        // TODO Log an error here?
        _ => None,
    };

    let action = action_payload.map(|payload| TaskActionInvocation {
        name: action.action_id.clone(),
        payload,
    });

    Ok(NodeResult {
        state: result,
        action,
        console,
    })
}

const ASYNC_FUNCTION_START: &str = r##"(async function() { "##;
const SYNC_FUNCTION_START: &str = r##"(function() { "##;
const FUNCTION_END: &str = r##" })()"##;

fn wrap_code(expr: &DataFlowJs) -> String {
    match expr.format {
        JsCodeFormat::Expression => format!(
            "{SYNC_FUNCTION_START}return {body}{FUNCTION_END}",
            body = expr.code
        ),
        JsCodeFormat::Function => format!(
            "{SYNC_FUNCTION_START}{body}{FUNCTION_END}",
            body = expr.code
        ),
        JsCodeFormat::AsyncFunction => format!(
            "{ASYNC_FUNCTION_START}{body}{FUNCTION_END}",
            body = expr.code
        ),
    }
}

#[cfg(target_family = "wasm")]
mod js {
    use super::*;

    #[derive(Debug, Deserialize, Serialize)]
    pub struct ConsoleMessage {
        level: i32,
        message: String,
    }

    pub(super) async fn run_js(
        task_name: &str,
        node_name: &str,
        expr: &DataFlowJs,
        current_state: serde_json::Value,
        input: NodeInput,
    ) -> Result<(serde_json::Value, Vec<ConsoleMessage>)> {
        let code = wrap_code(expr);
        let result = js_sys::eval(&code)?;

        Ok((serde_wasm_bindgen::from_value(result)?, Vec::new()))
    }
}

#[cfg(not(target_family = "wasm"))]
mod js {
    use super::*;
    use crate::scripting::{create_task_script_runtime, POOL};
    pub use ergo_js::ConsoleMessage;
    use ergo_js::Runtime;

    pub(super) async fn run_js(
        task_name: &str,
        node_name: &str,
        expr: &DataFlowJs,
        current_state: serde_json::Value,
        input: NodeInput,
    ) -> Result<(serde_json::Value, Vec<ConsoleMessage>)> {
        let name = format!("https://ergo/tasks/{task_name}/{node_name}.js");
        let wrapped = wrap_code(expr);

        POOL.run(move || async move {
            let mut runtime = create_task_script_runtime(true);
            set_up_env(&mut runtime, current_state, input).map_err(Error::TaskScriptSetup)?;

            let run_result = runtime
                .await_expression::<serde_json::Value>(&name, &wrapped)
                .await;
            let console = runtime.take_console_messages();
            match run_result {
                Ok(value) => Ok((value, console)),
                // TODO Generate a source map and use it to translate the code locations in the error.
                Err(error) => Err(Error::TaskScript { error, console }),
            }
        })
        .await
        .map_err(|e| match e {
            Error::TaskScript { error, console } => Error::DataflowScript {
                node: node_name.to_string(),
                error,
                console,
            },
            _ => e,
        })
    }

    fn set_up_env(
        runtime: &mut Runtime,
        current_state: serde_json::Value,
        input: NodeInput,
    ) -> Result<(), anyhow::Error> {
        runtime.set_global_value("last_value", &current_state)?;

        match input {
            NodeInput::Single(value) => runtime.set_global_value("value", &value)?,
            NodeInput::Multiple(values) => {
                for (key, value) in values {
                    runtime.set_global_value(&key, &value)?;
                }
            }
        }

        Ok(())
    }
}
