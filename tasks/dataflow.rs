use crate::{actions::TaskActionInvocations, Result};
use fxhash::FxHashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

mod dag;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DataFlowConfig {
    nodes: Vec<DataFlowNode>,
    /// The connection between nodes. This must be sorted.
    edges: Vec<DataFlowEdge>,
    toposorted: Vec<u32>,
    trigger_nodes: FxHashMap<String, DataFlowTrigger>,
}

impl DataFlowConfig {
    pub async fn evaluate_trigger(
        &self,
        state: &DataFlowState,
        trigger_id: &str,
        payload: &serde_json::Value,
    ) -> Result<(DataFlowState, TaskActionInvocations, bool)> {
        todo!()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DataFlowState {
    nodes: Vec<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DataFlowNode {
    function: DataFlowFunction,
    display_format: DataFlowOutputFormat,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DataFlowEdge {
    from: u32,
    to: u32,
    name: String,
}

impl PartialOrd for DataFlowEdge {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DataFlowEdge {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.from.cmp(&other.from) {
            std::cmp::Ordering::Equal => self.to.cmp(&other.to),
            x => x,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum DataFlowFunction {
    /// Don't alter the input data at all. Useful for graph and table nodes that aren't part of the
    /// main flow and just exist for display purposes.
    Identity,
    /// This node functions to take a task trigger and pass its data to other nodes.
    Trigger(DataFlowTrigger),
    /// Plain Text
    Text(DataFlowText),
    /// A single JavaScript expression
    JsExpression(DataFlowJsExpression),
    /// A JavaScript function body. This can be asynchronous
    JsFunction(DataFlowJsFunction),
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DataFlowText {
    body: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DataFlowTrigger {
    local_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DataFlowJsExpression {
    body: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DataFlowJsFunction {
    body: String,
    is_async: bool,
}

/// The output format for this node
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum DataFlowOutputFormat {
    /// A javascript object
    Js,
    /// Plain text
    Text,
    /// Render plain text as Markdown
    Markdown,
    /// Render plain text as Html
    Html,
    // To add: Table, Graph
}

impl DataFlowConfig {
    pub fn default_state(&self) -> DataFlowState {
        DataFlowState { nodes: Vec::new() }
    }
}
