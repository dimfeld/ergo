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
use tracing::{event, Level};

use self::dag::{toposort_nodes, NodeWalker};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DataFlowConfig {
    nodes: Vec<DataFlowNode>,
    /// The connection between nodes. This must be sorted.
    edges: Vec<DataFlowEdge>,
    toposorted: Vec<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataFlowLog {
    pub run: Vec<DataFlowNodeLog>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataFlowNodeLog {
    pub node: String,
    pub console: Vec<ConsoleMessage>,
}

impl DataFlowConfig {
    pub fn new(nodes: Vec<DataFlowNode>, edges: Vec<DataFlowEdge>) -> Result<Self> {
        let config = Self {
            toposorted: toposort_nodes(nodes.len(), &edges)?,
            nodes,
            edges,
        };

        Ok(config)
    }

    pub fn default_state(&self) -> DataFlowState {
        DataFlowState { nodes: Vec::new() }
    }

    pub async fn evaluate_trigger(
        &self,
        task_name: &str,
        mut state: DataFlowState,
        trigger_id: &str,
        payload: serde_json::Value,
    ) -> Result<(DataFlowState, Option<DataFlowLog>, TaskActionInvocations)> {
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
                &first_node.name,
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
                    let node_state = from_node.func.output(
                        state
                            .nodes
                            .get(edge.from as usize)
                            .unwrap_or(&serde_json::Value::Null),
                    );

                    (edge.name.clone(), node_state)
                })
                .collect::<FxHashMap<_, _>>();

            let node_state = state
                .nodes
                .get(node_idx)
                .unwrap_or(&serde_json::Value::Null);
            event!(Level::DEBUG, node=%node.name, state=?node_state, ?input, "Evaluating node");
            let result = node
                .func
                .execute(
                    task_name,
                    &node.name,
                    node_state,
                    NodeInput::Multiple(input),
                )
                .await?;

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

#[cfg(test)]
mod tests {
    use fxhash::FxHashMap;
    use serde_json::json;
    use wiremock::{
        matchers::{method, path, path_regex},
        Mock, MockServer, ResponseTemplate,
    };

    use super::{node::*, *};

    fn test_node(name: impl Into<String>, func: DataFlowNodeFunction) -> DataFlowNode {
        DataFlowNode {
            name: name.into(),
            func,
        }
    }

    async fn test_config() -> (MockServer, DataFlowConfig) {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(r"/doc/1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "code": 5 })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path(r"/doc/2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "code": 6 })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/doc/\d+"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "code": 0 })))
            .mount(&mock_server)
            .await;

        let nodes = vec![
            test_node(
                "trigger_a",
                DataFlowNodeFunction::Trigger(DataFlowTrigger {
                    local_id: "trigger1".to_string(),
                }),
            ),
            test_node(
                "trigger_b",
                DataFlowNodeFunction::Trigger(DataFlowTrigger {
                    local_id: "trigger2".to_string(),
                }),
            ),
            test_node(
                "add_one",
                DataFlowNodeFunction::Js(DataFlowJs {
                    code: "payload.value + 1".into(),
                    format: JsCodeFormat::Expression,
                }),
            ),
            test_node(
                "add_together",
                DataFlowNodeFunction::Js(DataFlowJs {
                    code: r##"let result = x.value + y.value;
                          return result;"##
                        .into(),
                    format: JsCodeFormat::Function,
                }),
            ),
            test_node(
                "fetch_given_value",
                DataFlowNodeFunction::Js(DataFlowJs {
                    format: JsCodeFormat::AsyncFunction,
                    code: format!(
                        r##"const response = await fetch({base_url}/doc/${{doc_id}});
                        const json = await response.json();
                        return {{ result: json.code }};"##,
                        base_url = mock_server.uri()
                    ),
                }),
            ),
            test_node(
                "email_label",
                DataFlowNodeFunction::Text(DataFlowText {
                    body: "The value:".into(),
                    render_as: TextRenderAs::PlainText,
                }),
            ),
            test_node(
                "send_email",
                DataFlowNodeFunction::Action(DataFlowAction {
                    action_id: "send_email".to_string(),
                    payload_code: DataFlowJs {
                        format: JsCodeFormat::Function,
                        code: r##"
                        if(code > 0) {
                            let contents = [label, code].join(' ');
                            return { contents };
                        }
                        "##
                        .into(),
                    },
                }),
            ),
        ];

        let name_indexes = nodes
            .iter()
            .enumerate()
            .map(|(i, node)| (node.name.clone(), i))
            .collect::<FxHashMap<_, _>>();

        let edges_by_name = [
            ("trigger_a", "add_one", "value"),
            ("trigger_a", "add_together", "x"),
            ("trigger_b", "add_together", "y"),
            ("add_together", "fetch_given_value", "doc_id"),
            ("email_label", "send_email", "label"),
            ("fetch_given_value", "send_email", "code"),
        ];

        let edges = edges_by_name
            .into_iter()
            .map(|(from, to, name)| {
                let from = name_indexes[from] as u32;
                let to = name_indexes[to] as u32;

                DataFlowEdge {
                    from,
                    to,
                    name: name.to_string(),
                }
            })
            .collect::<Vec<_>>();

        (mock_server, DataFlowConfig::new(nodes, edges).unwrap())
    }

    #[tokio::test]
    async fn test_run_nodes() {
        let (_server, config) = test_config().await;
        let state = config.default_state();

        let (state, log, actions) = config
            .evaluate_trigger("task", state, "trigger1", json!({ "value": 5 }))
            .await
            .unwrap();
        assert!(actions.is_empty());

        dbg!(log);
        dbg!(state);
    }
}
