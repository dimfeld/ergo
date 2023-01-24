use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod config;
mod dag;
mod node;
#[cfg(not(target_family = "wasm"))]
mod run;

pub use node::*;

pub use config::*;
pub use dag::toposort_nodes;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[cfg_attr(not(target_family = "wasm"), derive(JsonSchema))]
pub struct DataFlowState {
    /// The state is a set of JS values made safe for serialization by `devalue`. This allows objects such
    /// as Maps, Sets, Dates, etc. to be stored in the state.
    nodes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DataFlowEdge {
    from: u32,
    to: u32,
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
    use ergo_database::object_id::TaskTriggerId;
    use fxhash::FxHashMap;
    use serde_json::json;
    use wiremock::{
        matchers::{method, path, path_regex},
        Mock, MockServer, ResponseTemplate,
    };

    use crate::{actions::TaskActionInvocation, Error, Result};

    use super::{config::DataFlowConfig, *};

    fn edge_indexes_from_names(
        nodes: &[DataFlowNode],
        edges_by_name: &[(impl AsRef<str>, impl AsRef<str>)],
    ) -> Result<Vec<DataFlowEdge>> {
        let name_indexes = nodes
            .iter()
            .enumerate()
            .map(|(i, node)| (node.name.as_str(), i))
            .collect::<FxHashMap<_, _>>();

        edges_by_name
            .iter()
            .map(|(from, to)| {
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

                Ok(DataFlowEdge { from, to })
            })
            .collect::<Result<Vec<_>>>()
    }

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
    ) -> (MockServer, DataFlowConfig, TaskTriggerId, TaskTriggerId) {
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

        let code_bundle = format!(
            r##"
            (function() {{
                function __add_one({{ trigger_a }}) {{
                    return trigger_a.value + 1;
                }}

                function __add_together({{ trigger_a, trigger_b }}) {{
                    let x = trigger_a;
                    let y = trigger_b;
                    let result = {result_add};
                    console.log(`added together ${{result}}`);
                    return result;
                }}

                async function __fetch_given_value({{ add_together }}) {{
                    const response = await {fn_name}(`{base_url}/doc/${{add_together}}`);
                    const json = await response.json();
                    return {{ result: json.code }};
                }}

                function __send_email({{ fetch_given_value, email_label }}) {{
                    let code = fetch_given_value;
                    if(code.result) {{
                        let contents = [email_label, code.result].join(' ');
                        console.log('Sending the email:', contents);
                        return {{ contents }};
                    }}
                }}

                return {{
                    __add_one,
                    __add_together,
                    __fetch_given_value,
                    __send_email,
                }};
            }})()
            "##,
            result_add = if allow_null_inputs {
                "(x?.value ?? 0) + (y?.value ?? 0)"
            } else {
                "x.value + y.value"
            },
            base_url = mock_server.uri(),
            fn_name = if script_error { "bad_func" } else { "fetch" }
        );

        let trigger1_id = TaskTriggerId::new();
        let trigger2_id = TaskTriggerId::new();

        let nodes = vec![
            test_node(
                "trigger_a",
                false,
                DataFlowNodeFunction::Trigger(DataFlowTrigger {
                    task_trigger_id: trigger1_id,
                }),
            ),
            test_node(
                "trigger_b",
                false,
                DataFlowNodeFunction::Trigger(DataFlowTrigger {
                    task_trigger_id: trigger2_id,
                }),
            ),
            test_node(
                "add_one",
                false,
                DataFlowNodeFunction::Js(DataFlowJs {
                    func: "__add_one".to_string(),
                }),
            ),
            test_node(
                "add_together",
                allow_null_inputs,
                DataFlowNodeFunction::Js(DataFlowJs {
                    func: "__add_together".to_string(),
                }),
            ),
            test_node(
                "fetch_given_value",
                allow_null_inputs,
                DataFlowNodeFunction::Js(DataFlowJs {
                    func: "__fetch_given_value".to_string(),
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
                        func: "__send_email".to_string(),
                    },
                }),
            ),
        ];

        let edges_by_name = [
            ("trigger_a", "add_one"),
            ("trigger_a", "add_together"),
            ("trigger_b", "add_together"),
            ("add_together", "fetch_given_value"),
            ("email_label", "send_email"),
            ("fetch_given_value", "send_email"),
        ];

        let edges = edge_indexes_from_names(&nodes, &edges_by_name).expect("building edges");

        (
            mock_server,
            DataFlowConfig::new(nodes, edges, code_bundle, None).unwrap(),
            trigger1_id,
            trigger2_id,
        )
    }

    #[tokio::test]
    async fn test_run_nodes() {
        let (_server, config, trigger1, trigger2) = test_config(true, false).await;
        let state = config.default_state();

        println!("Sending 1 to trigger1");
        let (state, log, actions) = config
            .evaluate_trigger("task", state, trigger1, "trigger1", json!({ "value": 1 }))
            .await
            .unwrap();

        dbg!(&log);
        dbg!(&actions);
        dbg!(&state);

        assert_eq!(
            state.nodes,
            vec![
                r##"[{"value":1},1]"##,
                "",
                "[2]",
                "[1]",
                r##"[{"result":1},5]"##,
                "",
                r##"[{"contents":1},"The value: 5"]"##,
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
            .evaluate_trigger("task", state, trigger2, "trigger2", json!({ "value": -1 }))
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
                r##"[{"value":1},1]"##,
                r##"[{"value":1},-1]"##,
                "[2]",
                "[0]",
                r##"[{"result":1},0]"##,
                "",
                "[null]",
            ]
        );

        println!("Sending 2 to trigger2");
        let (state, log, actions) = config
            .evaluate_trigger("task", state, trigger2, "trigger2", json!({ "value": 2 }))
            .await
            .unwrap();

        dbg!(&log);
        dbg!(&actions);
        dbg!(&state);

        assert_eq!(
            state.nodes,
            vec![
                r##"[{"value":1},1]"##,
                r##"[{"value":1},2]"##,
                "[2]",
                "[3]",
                r##"[{"result":1},7]"##,
                "",
                r##"[{"contents":1},"The value: 7"]"##,
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
        let (_server, config, trigger1, trigger2) = test_config(false, false).await;
        let state = config.default_state();

        println!("Sending 1 to trigger1");
        let (state, log, actions) = config
            .evaluate_trigger("task", state, trigger1, "trigger1", json!({ "value": 1 }))
            .await
            .unwrap();

        assert_eq!(
            state.nodes,
            vec![r##"[{"value":1},1]"##, "", "[2]", "", "", "", "",]
        );

        assert!(actions.is_empty());
        assert!(log.is_none());

        println!("Sending 2 to trigger2");
        let (state, log, actions) = config
            .evaluate_trigger("task", state, trigger2, "trigger2", json!({ "value": 2 }))
            .await
            .unwrap();

        assert_eq!(
            state.nodes,
            vec![
                r##"[{"value":1},1]"##,
                r##"[{"value":1},2]"##,
                "[2]",
                "[3]",
                r##"[{"result":1},7]"##,
                "",
                r##"[{"contents":1},"The value: 7"]"##,
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
        let (_server, config, trigger1, trigger2) = test_config(true, true).await;
        let state = config.default_state();

        println!("Sending 1 to trigger1");
        let err = config
            .evaluate_trigger("task", state, trigger1, "trigger1", json!({ "value": 1 }))
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
