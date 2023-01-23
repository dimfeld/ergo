use crate::{actions::TaskActionInvocation, Error, Result};
#[cfg(not(target_family = "wasm"))]
pub use ergo_js::ConsoleMessage;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[cfg(not(target_family = "wasm"))]
pub use node_result::*;

#[cfg(not(target_family = "wasm"))]
mod node_result {
    pub use ergo_js::ConsoleMessage;

    use crate::actions::TaskActionInvocation;

    #[derive(Debug)]
    pub(crate) struct NodeResult {
        pub state: String,
        pub action: Option<TaskActionInvocation>,
        pub console: Vec<ConsoleMessage>,
    }

    impl NodeResult {
        pub fn empty() -> Self {
            Self {
                state: String::new(),
                action: None,
                console: Vec::new(),
            }
        }
    }

    impl From<(String, Vec<ConsoleMessage>)> for NodeResult {
        fn from((state, console): (String, Vec<ConsoleMessage>)) -> Self {
            Self {
                state,
                action: None,
                console,
            }
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
    /// The name of the function in the compiled code that stores this node.
    pub func: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DataFlowAction {
    pub action_id: String,
    pub payload_code: DataFlowJs,
}

impl DataFlowNodeFunction {
    #[cfg(not(target_family = "wasm"))]
    pub(super) async fn execute(
        &self,
        task_name: &str,
        node_name: &str,
        runner: &super::run::DataFlowRunner,
        null_check_nodes: &[&str],
        input: Option<serde_json::Value>,
    ) -> Result<Option<NodeResult>> {
        match self {
            Self::Js(expr) => run_js(task_name, node_name, runner, null_check_nodes, expr)
                .await
                .map(|r| r.map(NodeResult::from)),
            Self::Action(expr) => {
                evaluate_action_node(task_name, node_name, runner, null_check_nodes, expr).await
            }
            Self::Trigger(_) => {
                let value = input.unwrap_or_default();
                let store_state = runner.set_node_state(node_name, &value).await?;
                Ok(Some(NodeResult {
                    state: store_state,
                    action: None,
                    console: Vec::new(),
                }))
            }
            Self::Text(t) => {
                let state = runner.set_node_state(node_name, &json!(t.body)).await?;
                Ok(Some(NodeResult {
                    state,
                    action: None,
                    console: Vec::new(),
                }))
            }
            Self::Table | Self::Graph => Ok(None),
        }
    }

    pub(super) fn persist_output(&self) -> bool {
        match self {
            Self::Js(_) | Self::Action(_) | Self::Trigger(_) | Self::Table | Self::Graph => true,
            Self::Text(_) => false,
        }
    }
}

#[cfg(not(target_family = "wasm"))]
async fn evaluate_action_node(
    task_name: &str,
    node_name: &str,
    runner: &super::run::DataFlowRunner,
    null_check_nodes: &[&str],
    action: &DataFlowAction,
) -> Result<Option<NodeResult>> {
    let result = run_js(
        task_name,
        node_name,
        runner,
        null_check_nodes,
        &action.payload_code,
    )
    .await
    .map_err(|e| match e {
        Error::TaskScript { error, console } => Error::DataflowScript {
            node: node_name.to_string(),
            error,
            console,
        },
        _ => e,
    })?;

    let Some((result, console)) = result else {
        return Ok(None);
    };

    let action_payload =
        runner
            .get_raw_state(node_name)
            .await
            .map_err(|e| Error::DataflowGetStateError {
                node: node_name.to_string(),
                error: e,
            })?;

    let action = match action_payload {
        serde_json::Value::Null => None,
        serde_json::Value::Object(_) => Some(TaskActionInvocation {
            name: action.action_id.clone(),
            payload: action_payload,
        }),
        // TODO Log an error here?
        _ => None,
    };

    Ok(Some(NodeResult {
        state: result,
        action,
        console,
    }))
}

#[cfg(not(target_family = "wasm"))]
async fn run_js(
    task_name: &str,
    node_name: &str,
    runner: &super::run::DataFlowRunner,
    null_check_nodes: &[&str],
    expr: &DataFlowJs,
) -> Result<Option<(String, Vec<ConsoleMessage>)>> {
    runner
        .run_node(task_name, node_name, &expr.func, null_check_nodes)
        .await
        .map(|(result, console)| {
            if result.is_empty() {
                None
            } else {
                Some((result, console))
            }
        })
        .map_err(|e| match e {
            Error::TaskScript { error, console } => Error::DataflowScript {
                node: node_name.to_string(),
                error,
                console,
            },
            _ => e,
        })
}
