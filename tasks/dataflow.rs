use crate::{
    actions::TaskActionInvocations,
    dataflow::node::{DataFlowNodeFunction, NodeInput},
    Error, Result,
};
use ergo_js::ConsoleMessage;
use fxhash::FxHashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod dag;
mod node;

pub use node::DataFlowNode;

use self::dag::NodeWalker;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DataFlowConfig {
    nodes: Vec<DataFlowNode>,
    /// The connection between nodes. This must be sorted.
    edges: Vec<DataFlowEdge>,
    toposorted: Vec<u32>,
}

pub struct DataFlowLog {
    node: String,
    console: Vec<ConsoleMessage>,
}

impl DataFlowConfig {
    pub async fn evaluate_trigger(
        &self,
        task_name: &str,
        mut state: DataFlowState,
        trigger_id: &str,
        payload: serde_json::Value,
    ) -> Result<(DataFlowState, TaskActionInvocations, bool)> {
        let trigger_node = self
            .nodes
            .iter()
            .position(|node| match &node.func {
                DataFlowNodeFunction::Trigger(trigger) => trigger.local_id == trigger_id,
                _ => false,
            })
            .ok_or_else(|| Error::TaskTriggerNotFound(trigger_id.to_string()))?;

        let mut walker = NodeWalker::starting_from(self, trigger_node as u32)?;

        // Directly send the payload into the first node. The rest of the nodes have their state built the
        // normal way.
        let first_node_idx = walker.next().unwrap();
        let first_node = &self.nodes[first_node_idx as usize];
        let new_state = first_node
            .func
            .execute(
                task_name,
                &serde_json::Value::Null,
                NodeInput::Single(payload),
            )
            .await?;

        if first_node.func.persist_output() {
            state.nodes[first_node_idx] = new_state.state;
        }

        let mut logs = Vec::new();
        let mut actions = TaskActionInvocations::new();

        for node_idx in walker {
            let node = &self.nodes[node_idx as usize];

            // Gather the inputs for the node
            let input = self
                .edges
                .iter()
                .filter(|edge| edge.to as usize == node_idx)
                .map(|edge| {
                    let from_node = &self.nodes[edge.from as usize];
                    let node_state = from_node.func.output(&state.nodes[edge.from as usize]);

                    (edge.name.clone(), node_state)
                })
                .collect::<FxHashMap<_, _>>();

            let result = node
                .func
                .execute(
                    task_name,
                    &state.nodes[node_idx],
                    NodeInput::Multiple(input),
                )
                .await?;

            if !result.console.is_empty() {
                logs.push(DataFlowLog {
                    node: node.name.clone(),
                    console: result.console,
                });
            }

            if node.func.persist_output() {
                state.nodes[node_idx] = result.state;
            }

            if let Some(action) = result.action {
                actions.push(action);
            }
        }

        todo!();
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DataFlowState {
    nodes: Vec<serde_json::Value>,
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

impl DataFlowConfig {
    pub fn default_state(&self) -> DataFlowState {
        DataFlowState { nodes: Vec::new() }
    }
}
