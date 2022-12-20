use crate::{actions::TaskActionInvocations, Error, Result};
use ergo_js::ConsoleMessage;
use fxhash::FxHashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod dag;
mod node;

pub use node::*;
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
        if state.nodes.len() != self.nodes.len() {
            state
                .nodes
                .resize(self.nodes.len(), serde_json::Value::Null);
        }

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
        let first_node = &self.nodes[first_node_idx];
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
            let node = &self.nodes[node_idx];

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

                    if !node.allow_null_inputs && node_state == serde_json::Value::Null {
                        // Return an Err just because it's a convenient way to short-circuit the
                        // iteration.
                        Err(())
                    } else {
                        Ok((edge.name.clone(), node_state))
                    }
                })
                .collect::<Result<FxHashMap<_, _>, ()>>();

            let input = match input {
                Ok(input) => input,
                // This just means that the node is not running because one of its inputs is null.
                // Not a real error, so just continue to the next node.
                Err(()) => continue,
            };

            let node_state = state
                .nodes
                .get(node_idx)
                .unwrap_or(&serde_json::Value::Null);
            event!(Level::DEBUG, node=%node.name, state=?node_state, ?input, "Evaluating node");
            dbg!(&node);
            dbg!(&node_state);
            dbg!(&input);
            let result = node
                .func
                .execute(
                    task_name,
                    &node.name,
                    node_state,
                    NodeInput::Multiple(input),
                )
                .await?;
            dbg!(&result);

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

pub fn edge_indexes_from_names(
    nodes: &[DataFlowNode],
    edges_by_name: &[(impl AsRef<str>, impl AsRef<str>, impl ToString)],
) -> Result<Vec<DataFlowEdge>> {
    let name_indexes = nodes
        .iter()
        .enumerate()
        .map(|(i, node)| (node.name.as_str(), i))
        .collect::<FxHashMap<_, _>>();

    edges_by_name
        .iter()
        .map(|(from, to, name)| {
            let from = name_indexes
                .get(from.as_ref())
                .copied()
                .ok_or_else(|| Error::MissingDataFlowNodeName(from.as_ref().to_string()))?
                as u32;
            let to = name_indexes
                .get(to.as_ref())
                .copied()
                .ok_or_else(|| Error::MissingDataFlowNodeName(to.as_ref().to_string()))?
                as u32;

            Ok(DataFlowEdge {
                from,
                to,
                name: name.to_string(),
            })
        })
        .collect::<Result<Vec<_>>>()
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::{
        matchers::{method, path, path_regex},
        Mock, MockServer, ResponseTemplate,
    };

    use crate::actions::TaskActionInvocation;

    use super::*;

    fn test_node(
        name: impl Into<String>,
        allow_null_inputs: bool,
        func: DataFlowNodeFunction,
    ) -> DataFlowNode {
        DataFlowNode {
            name: name.into(),
            allow_null_inputs,
            func,
        }
    }

    async fn test_config(
        allow_null_inputs: bool,
        script_error: bool,
    ) -> (MockServer, DataFlowConfig) {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(r"/doc/1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "code": 5 })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path(r"/doc/3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "code": 7 })))
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
                false,
                DataFlowNodeFunction::Trigger(DataFlowTrigger {
                    local_id: "trigger1".to_string(),
                }),
            ),
            test_node(
                "trigger_b",
                false,
                DataFlowNodeFunction::Trigger(DataFlowTrigger {
                    local_id: "trigger2".to_string(),
                }),
            ),
            test_node(
                "add_one",
                false,
                DataFlowNodeFunction::Js(DataFlowJs {
                    code: "value.value + 1".into(),
                    format: JsCodeFormat::Expression,
                }),
            ),
            test_node(
                "add_together",
                allow_null_inputs,
                DataFlowNodeFunction::Js(DataFlowJs {
                    code: format!(
                        r##"
                        let result = {result_add};
                        console.log(`added together ${{result}}`);
                        return result;"##,
                        result_add = if allow_null_inputs {
                            "(x?.value ?? 0) + (y?.value ?? 0)"
                        } else {
                            "x.value + y.value"
                        }
                    ),
                    format: JsCodeFormat::Function,
                }),
            ),
            test_node(
                "fetch_given_value",
                allow_null_inputs,
                DataFlowNodeFunction::Js(DataFlowJs {
                    format: JsCodeFormat::AsyncFunction,
                    code: format!(
                        r##"const response = await {fn_name}(`{base_url}/doc/${{doc_id}}`);
                        const json = await response.json();
                        return {{ result: json.code }};"##,
                        base_url = mock_server.uri(),
                        fn_name = if script_error { "bad_func" } else { "fetch" }
                    ),
                }),
            ),
            test_node(
                "email_label",
                false,
                DataFlowNodeFunction::Text(DataFlowText {
                    body: "The value:".into(),
                    render_as: TextRenderAs::PlainText,
                }),
            ),
            test_node(
                "send_email",
                allow_null_inputs,
                DataFlowNodeFunction::Action(DataFlowAction {
                    action_id: "send_email".to_string(),
                    payload_code: DataFlowJs {
                        format: JsCodeFormat::Function,
                        code: r##"
                        if(code.result) {
                            let contents = [label, code.result].join(' ');
                            console.log('Sending the email:', contents);
                            return { contents };
                        }
                        "##
                        .into(),
                    },
                }),
            ),
        ];

        let edges_by_name = [
            ("trigger_a", "add_one", "value"),
            ("trigger_a", "add_together", "x"),
            ("trigger_b", "add_together", "y"),
            ("add_together", "fetch_given_value", "doc_id"),
            ("email_label", "send_email", "label"),
            ("fetch_given_value", "send_email", "code"),
        ];

        let edges = edge_indexes_from_names(&nodes, &edges_by_name).expect("building edges");

        (mock_server, DataFlowConfig::new(nodes, edges).unwrap())
    }

    #[tokio::test]
    async fn test_run_nodes() {
        let (_server, config) = test_config(true, false).await;
        let state = config.default_state();

        println!("Sending 1 to trigger1");
        let (state, log, actions) = config
            .evaluate_trigger("task", state, "trigger1", json!({ "value": 1 }))
            .await
            .unwrap();

        dbg!(&log);
        dbg!(&actions);
        dbg!(&state);

        assert_eq!(
            state.nodes,
            vec![
                json!({ "value": 1 }),
                json!(null),
                json!(2),
                json!(1),
                json!({ "result": 5 }),
                json!(null),
                json!({ "contents": "The value: 5" }),
            ]
        );
        assert_eq!(
            actions.as_slice(),
            vec![TaskActionInvocation {
                name: "send_email".to_string(),
                payload: json!({ "contents": "The value: 5" }),
            }]
            .as_slice()
        );

        let log = log.expect("log exists");

        assert_eq!(log.run[0].node, "add_together");
        assert_eq!(log.run[0].console.len(), 1);
        assert_eq!(log.run[0].console[0].message, "added together 1\n");

        assert_eq!(log.run[1].node, "send_email");
        assert_eq!(log.run[1].console.len(), 1);
        assert_eq!(
            log.run[1].console[0].message,
            "Sending the email: The value: 5\n"
        );

        println!("Sending -1 to trigger2");
        let (state, log, actions) = config
            .evaluate_trigger("task", state, "trigger2", json!({ "value": -1 }))
            .await
            .unwrap();

        dbg!(&log);
        dbg!(&actions);
        dbg!(&state);

        // This should end up with the value sent to the email action being 0, so the code there won't
        // send it.
        assert!(actions.is_empty());

        let log = log.expect("log exists");
        assert_eq!(log.run[0].node, "add_together");
        assert_eq!(log.run[0].console.len(), 1);
        assert_eq!(log.run[0].console[0].message, "added together 0\n");

        assert_eq!(
            state.nodes,
            vec![
                json!({ "value": 1 }),
                json!({ "value": -1 }),
                json!(2),
                json!(0),
                json!({ "result": 0 }),
                json!(null),
                json!(null),
            ]
        );

        println!("Sending 2 to trigger2");
        let (state, log, actions) = config
            .evaluate_trigger("task", state, "trigger2", json!({ "value": 2 }))
            .await
            .unwrap();

        dbg!(&log);
        dbg!(&actions);
        dbg!(&state);

        assert_eq!(
            state.nodes,
            vec![
                json!({ "value": 1 }),
                json!({ "value": 2 }),
                json!(2),
                json!(3),
                json!({ "result": 7 }),
                json!(null),
                json!({ "contents": "The value: 7" }),
            ]
        );
        assert_eq!(
            actions.as_slice(),
            vec![TaskActionInvocation {
                name: "send_email".to_string(),
                payload: json!({ "contents": "The value: 7" }),
            }]
            .as_slice()
        );

        let log = log.expect("log exists");

        assert_eq!(log.run[0].node, "add_together");
        assert_eq!(log.run[0].console.len(), 1);
        assert_eq!(log.run[0].console[0].message, "added together 3\n");

        assert_eq!(log.run[1].node, "send_email");
        assert_eq!(log.run[1].console.len(), 1);
        assert_eq!(
            log.run[1].console[0].message,
            "Sending the email: The value: 7\n"
        );
    }

    #[tokio::test]
    async fn allow_null_inputs_false() {
        let (_server, config) = test_config(false, false).await;
        let state = config.default_state();

        println!("Sending 1 to trigger1");
        let (state, log, actions) = config
            .evaluate_trigger("task", state, "trigger1", json!({ "value": 1 }))
            .await
            .unwrap();

        assert_eq!(
            state.nodes,
            vec![
                json!({ "value": 1 }),
                json!(null),
                json!(2),
                json!(null),
                json!(null),
                json!(null),
                json!(null),
            ]
        );

        assert!(actions.is_empty());
        assert!(log.is_none());

        println!("Sending 2 to trigger2");
        let (state, log, actions) = config
            .evaluate_trigger("task", state, "trigger2", json!({ "value": 2 }))
            .await
            .unwrap();

        assert_eq!(
            state.nodes,
            vec![
                json!({ "value": 1 }),
                json!({ "value": 2 }),
                json!(2),
                json!(3),
                json!({ "result": 7 }),
                json!(null),
                json!({ "contents": "The value: 7" }),
            ]
        );
        assert_eq!(
            actions.as_slice(),
            vec![TaskActionInvocation {
                name: "send_email".to_string(),
                payload: json!({ "contents": "The value: 7" }),
            }]
            .as_slice()
        );

        let log = log.expect("log exists");

        assert_eq!(log.run[0].node, "add_together");
        assert_eq!(log.run[0].console.len(), 1);
        assert_eq!(log.run[0].console[0].message, "added together 3\n");

        assert_eq!(log.run[1].node, "send_email");
        assert_eq!(log.run[1].console.len(), 1);
        assert_eq!(
            log.run[1].console[0].message,
            "Sending the email: The value: 7\n"
        );
    }

    #[tokio::test]
    async fn bad_script() {
        let (_server, config) = test_config(true, true).await;
        let state = config.default_state();

        println!("Sending 1 to trigger1");
        let err = config
            .evaluate_trigger("task", state, "trigger1", json!({ "value": 1 }))
            .await
            .expect_err("should have failed");

        if let Error::DataflowScript { node, error, .. } = err {
            assert_eq!(node, "fetch_given_value");
            assert!(error
                .to_string()
                .contains("ReferenceError: bad_func is not defined"));
        } else {
            panic!("Unexpected error: {:?}", err);
        }
    }
}
