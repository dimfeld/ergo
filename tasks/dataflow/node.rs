use crate::{
    actions::TaskActionInvocation,
    scripting::{create_task_script_runtime, POOL},
    Error, Result,
};
use ergo_js::{ConsoleMessage, Runtime};
use fxhash::FxHashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DataFlowNode {
    pub name: String,
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

#[derive(Debug)]
pub(super) struct NodeResult {
    pub state: serde_json::Value,
    pub action: Option<TaskActionInvocation>,
    pub console: Vec<ConsoleMessage>,
}

impl NodeResult {
    fn empty() -> Self {
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

impl DataFlowNodeFunction {
    pub(super) async fn execute(
        &self,
        task_name: &str,
        current_state: &serde_json::Value,
        input: NodeInput,
    ) -> Result<NodeResult> {
        match self {
            Self::Js(expr) => run_js(task_name, expr, current_state.clone(), input)
                .await
                .map(NodeResult::from),
            Self::Action(expr) => {
                evaluate_action_node(task_name, expr, current_state.clone(), input).await
            }
            Self::Text(_) | Self::Table | Self::Graph | Self::Trigger(_) => Ok(NodeResult::empty()),
        }
    }

    pub(super) fn persist_output(&self) -> bool {
        match self {
            Self::Js(_) | Self::Action(_) | Self::Text(_) => true,
            Self::Table | Self::Graph | Self::Trigger(_) => false,
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
    name: &str,
    action: &DataFlowAction,
    current_state: serde_json::Value,
    input: NodeInput,
) -> Result<NodeResult> {
    let (result, console) = run_js(name, &action.payload_code, current_state, input).await?;

    let action = match &result {
        serde_json::Value::Null => None,
        serde_json::Value::Object(_) => Some(TaskActionInvocation {
            name: action.action_id.clone(),
            payload: result.clone(),
        }),
        // TODO Return an error here?
        _ => None,
    };

    Ok(NodeResult {
        state: result,
        action,
        console,
    })
}

const ASYNC_FUNCTION_START: &str = r##"(async function() { "##;
const FUNCTION_END: &str = r##" }"##;
const SYNC_FUNCTION_START: &str = r##"(function() { "##;

async fn run_js(
    name: &str,
    expr: &DataFlowJs,
    current_state: serde_json::Value,
    input: NodeInput,
) -> Result<(serde_json::Value, Vec<ConsoleMessage>)> {
    let main_url = url::Url::parse(&format!("https://ergo/tasks/{}.js", name))
        .map_err(|e| Error::TaskScriptSetup(e.into()))?;

    let wrapped = match expr.format {
        JsCodeFormat::Expression => expr.code.clone(),
        JsCodeFormat::Function => format!(
            "{SYNC_FUNCTION_START}{body}{FUNCTION_END}",
            body = expr.code
        ),
        JsCodeFormat::AsyncFunction => format!(
            "{ASYNC_FUNCTION_START}{body}{FUNCTION_END}",
            body = expr.code
        ),
    };

    POOL.run(move || async move {
        let mut runtime = create_task_script_runtime(true);
        set_up_env(&mut runtime, current_state, input).map_err(Error::TaskScriptSetup)?;

        let run_result = runtime.run_main_module(main_url, wrapped).await;
        let console = runtime.take_console_messages();
        match run_result {
            Ok(()) => {
                let result_value: serde_json::Value = runtime
                    .await_global_value("__ergo_result")
                    .await
                    // TODO Probably need to handle the unresolved promise error here.
                    .ok()
                    .unwrap_or_default()
                    .unwrap_or_default();

                Ok((result_value, console))
            }
            // TODO Generate a source map and use it to translate the code locations in the error.
            Err(error) => Err(Error::TaskScript { error, console }),
        }
    })
    .await
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
