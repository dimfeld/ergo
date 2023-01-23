use crate::{actions::TaskActionInvocations, Error, Result};
use fxhash::FxHashSet;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::{event, Level};

use super::{
    dag::NodeWalker, toposort_nodes, DataFlowEdge, DataFlowNode, DataFlowNodeFunction,
    DataFlowState,
};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DataFlowConfig {
    pub(super) nodes: Vec<DataFlowNode>,
    /// The connection between nodes. This must be sorted.
    pub(super) edges: Vec<DataFlowEdge>,
    /// The compiled JavaScript for all the nodes and their dependencies, bundled as an IIFE
    pub(super) compiled: String,
    /// A source map for the compiled JavaScript
    pub(super) map: Option<String>,
    pub(super) toposorted: Vec<u32>,
}

impl DataFlowConfig {
    pub fn new(
        nodes: Vec<DataFlowNode>,
        edges: Vec<DataFlowEdge>,
        compiled: String,
        map: Option<String>,
    ) -> Result<Self> {
        let config = Self {
            toposorted: toposort_nodes(nodes.len(), &edges)?,
            compiled,
            map,
            nodes,
            edges,
        };

        Ok(config)
    }

    pub fn default_state(&self) -> DataFlowState {
        DataFlowState { nodes: Vec::new() }
    }

    #[cfg(not(target_family = "wasm"))]
    pub async fn evaluate_trigger(
        &self,
        task_name: &str,
        mut state: DataFlowState,
        trigger_id: &str,
        payload: serde_json::Value,
    ) -> Result<(
        DataFlowState,
        Option<super::run::DataFlowLog>,
        TaskActionInvocations,
    )> {
        use crate::dataflow::run::DataFlowNodeLog;

        use super::run::{DataFlowLog, DataFlowRunner};

        if state.nodes.len() != self.nodes.len() {
            state.nodes.resize_with(self.nodes.len(), String::new);
        }

        let mut to_run = FxHashSet::default();

        let runner = DataFlowRunner::new(self, &state).await?;

        let trigger_node = self
            .nodes
            .iter()
            .position(|node| match &node.func {
                DataFlowNodeFunction::Trigger(trigger) => trigger.local_id == trigger_id,
                _ => false,
            })
            .ok_or_else(|| Error::TaskTriggerNotFound(trigger_id.to_string()))?;

        to_run.insert(trigger_node);
        let mut walker = NodeWalker::starting_from(self, trigger_node as u32)?;

        // Directly send the payload into the first node. The rest of the nodes have their state built the
        // normal way.
        let first_node_idx = walker.next().unwrap();
        let first_node = &self.nodes[first_node_idx];
        let new_state = first_node
            .func
            .execute(task_name, &first_node.name, &runner, &[], Some(payload))
            .await?;

        let Some(new_state) = new_state else {
            return Ok((state, None, TaskActionInvocations::default()));
        };

        if first_node.func.persist_output() {
            state.nodes[first_node_idx] = new_state.state;
        }

        // Add all directly connected nodes to the list of nodes to run.
        self.edges
            .iter()
            .filter(|edge| edge.from as usize == trigger_node)
            .for_each(|edge| {
                to_run.insert(edge.to as usize);
            });

        let mut logs = Vec::new();
        let mut actions = TaskActionInvocations::new();

        for node_idx in walker {
            if !to_run.contains(&node_idx) {
                // This node doesn't depend on anything that actually ran, so skip it.
                continue;
            }

            let node = &self.nodes[node_idx];

            let null_check_nodes = if node.allow_null_inputs {
                vec![]
            } else {
                self.edges
                    .iter()
                    .filter(|edge| edge.to as usize == node_idx)
                    .map(|edge| self.nodes[edge.from as usize].name.as_str())
                    .collect()
            };

            event!(Level::DEBUG, node=%node.name, state=?state, "Evaluating node");
            dbg!(&node);
            dbg!(&state);
            let result = node
                .func
                .execute(task_name, &node.name, &runner, &null_check_nodes, None)
                .await?;
            dbg!(&result);

            let Some(result) = result else { continue; };

            // Add all directly connected nodes to the list of nodes to run.
            self.edges
                .iter()
                .filter(|edge| edge.from as usize == node_idx)
                .for_each(|edge| {
                    to_run.insert(edge.to as usize);
                });

            if !result.console.is_empty() {
                logs.push(DataFlowNodeLog {
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

        let log_output = if logs.is_empty() {
            None
        } else {
            Some(DataFlowLog { run: logs })
        };

        Ok((state, log_output, actions))
    }
}
