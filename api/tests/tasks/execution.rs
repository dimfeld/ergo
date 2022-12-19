use std::{borrow::Cow, time::Duration};

use anyhow::Result;
use ergo_api::routes::{
    actions::ActionPayload,
    inputs::InputPayload,
    tasks::{InputsLogEntry, TaskActionInput, TaskInput, TaskTriggerInput},
};
use ergo_database::object_id::{ActionId, InputId, OrgId, TaskId};
use ergo_tasks::{
    actions::{
        execute::ScriptOrTemplate,
        template::{TemplateField, TemplateFieldFormat},
        Action, ActionStatus,
    },
    dataflow::{
        edge_indexes_from_names, DataFlowAction, DataFlowConfig, DataFlowJs, DataFlowNode,
        DataFlowNodeFunction, DataFlowState, DataFlowTrigger, JsCodeFormat,
    },
    inputs::{Input, InputStatus},
    scripting::{TaskJsConfig, TaskJsState},
    state_machine::{
        ActionInvokeDef, ActionPayloadBuilder, EventHandler, StateDefinition, StateMachine,
        StateMachineData,
    },
    TaskConfig, TaskState,
};
use fxhash::FxHashMap;
use serde_json::json;
use smallvec::smallvec;
use tracing::{event, Level};
use uuid::Uuid;
use wiremock::{
    matchers::{body_json, method, path},
    Mock, MockServer, ResponseTemplate,
};

use crate::common::{run_app_test, TestApp, TestUser};

#[allow(dead_code)]
struct BootstrappedData {
    org: OrgId,
    user: TestUser,
    url_input: Input,
    url_input_payload: InputPayload,
    url_input_id: InputId,
    string_input: Input,
    string_input_payload: InputPayload,
    string_input_id: InputId,
    script_input: Input,
    script_input_payload: InputPayload,
    script_input_id: InputId,
    script_action: Action,
    script_action_payload: ActionPayload,
    script_action_id: ActionId,
    http_action: Action,
    http_action_payload: ActionPayload,
    http_action_id: ActionId,
}

async fn bootstrap(app: &TestApp) -> Result<BootstrappedData> {
    let org = app.add_org("user org").await?;
    let user = app.add_user(&org, "user 1").await?;

    let url_input_id = InputId::new();
    let url_input_payload = InputPayload {
        name: "url".to_string(),
        description: None,
        input_category_id: None,
        payload_schema: json!({
          "$schema": "http://json-schema.org/draft-07/schema",
          "$id": "http://ergo.dev/inputs/url.json",
          "type": "object",
          "required": [
              "url"
          ],
          "properties": {
              "url": {
                  "type": "string"
              }
          },
          "additionalProperties": true
        }),
    };

    let url_input = app
        .admin_user
        .client
        .put_input(&url_input_id, &url_input_payload)
        .await
        .expect("bootstrap: url_input_payload");

    let string_input_id = InputId::new();
    let string_input_payload = InputPayload {
        name: "string".to_string(),
        description: None,
        input_category_id: None,
        payload_schema: json!({
          "$schema": "http://json-schema.org/draft-07/schema",
          "$id": "http://ergo.dev/inputs/string.json",
          "type": "object",
          "required": [
              "value"
          ],
          "properties": {
              "value": {
                  "type": "string"
              }
          },
          "additionalProperties": true
        }),
    };

    let string_input = app
        .admin_user
        .client
        .put_input(&string_input_id, &string_input_payload)
        .await
        .expect("bootstrap: string_input_payload");

    let script_input_id = InputId::new();
    let script_input_payload = InputPayload {
        name: "run script".to_string(),
        description: None,
        input_category_id: None,
        payload_schema: json!({
          "$schema": "http://json-schema.org/draft-07/schema",
          "$id": "http://ergo.dev/inputs/script.json",
          "type": "object",
          "required": [
              "script"
          ],
          "properties": {
              "script": {
                  "type": "string"
              }
          },
          "additionalProperties": true
        }),
    };

    let script_input = app
        .admin_user
        .client
        .put_input(&script_input_id, &script_input_payload)
        .await
        .expect("bootstrap: writing script_input_payload");

    let script_action_id = ActionId::new();
    let script_action_payload = ActionPayload {
        action_category_id: app.base_action_category.clone(),
        name: "Run script".to_string(),
        description: None,
        executor_id: "js".to_string(),
        executor_template: ScriptOrTemplate::Template(vec![
            ("name".to_string(), json!("a script")),
            ("script".to_string(), json!("{{script}}")),
        ]),
        template_fields: vec![TemplateField {
            name: Cow::from("script"),
            format: TemplateFieldFormat::string_without_default(),
            optional: false,
            description: None,
        }]
        .into(),
        account_required: false,
        account_types: vec![],
        postprocess_script: None,
        timeout: None,
    };

    let script_action = app
        .admin_user
        .client
        .put_action(&script_action_id, &script_action_payload)
        .await
        .expect("bootstrap: writing script_action_payload");

    let http_action_id = ActionId::new();
    let http_action_payload = ActionPayload {
        action_category_id: app.base_action_category.clone(),
        name: "Send request".to_string(),
        description: None,
        executor_id: "http".to_string(),
        timeout: None,
        postprocess_script: None,
        account_types: vec![],
        account_required: false,
        executor_template: ScriptOrTemplate::Template(vec![
            ("url".to_string(), json!("{{url}}")),
            ("json".to_string(), json!("{{/payload}}")),
            ("method".to_string(), json!("POST")),
        ]),
        template_fields: vec![
            TemplateField {
                name: Cow::from("url"),
                format: TemplateFieldFormat::string_without_default(),
                optional: false,
                description: None,
            },
            TemplateField {
                name: Cow::from("payload"),
                format: TemplateFieldFormat::Object {
                    nested: true,
                    default: Cow::from("{}"),
                },
                optional: false,
                description: None,
            },
        ]
        .into(),
    };
    let http_action = app
        .admin_user
        .client
        .put_action(&http_action_id, &http_action_payload)
        .await
        .expect("bootstrap: writing http_action_payload");

    Ok(BootstrappedData {
        org,
        user,
        url_input,
        url_input_payload,
        url_input_id,
        string_input,
        string_input_payload,
        string_input_id,
        script_input,
        script_input_payload,
        script_input_id,
        script_action,
        script_action_payload,
        script_action_id,
        http_action,
        http_action_payload,
        http_action_id,
    })
}

async fn bootstrap_state_machine_task(base: &BootstrappedData) -> (TaskId, TaskInput) {
    let state_machine_task = TaskInput {
        name: "Run script".to_string(),
        description: None,
        alias: Some("run_script".to_string()),
        enabled: true,
        state: Some(TaskState::StateMachine(smallvec![StateMachineData {
            state: "initial".to_string(),
            context: json!(null)
        }])),

        source: json!(null),
        compiled: TaskConfig::StateMachine(smallvec![StateMachine {
            name: "run_action".to_string(),
            description: None,
            initial: "initial".to_string(),
            on: smallvec![],
            states: [(
                "initial".to_string(),
                StateDefinition {
                    on: smallvec![EventHandler {
                        trigger_id: "run".to_string(),
                        target: None,
                        actions: Some(vec![ActionInvokeDef {
                            task_action_local_id: "run".to_string(),
                            data: ActionPayloadBuilder::Script(
                                "return { script: payload.script }".to_string()
                            ),
                        }])
                    }],
                    description: None,
                }
            )]
            .into_iter()
            .collect::<FxHashMap<_, _>>()
        }]),

        actions: vec![(
            "run".to_string(),
            TaskActionInput {
                name: "Run the script".to_string(),
                action_id: base.script_action_id.clone(),
                account_id: None,
                action_template: None,
            },
        )]
        .into_iter()
        .collect::<FxHashMap<_, _>>(),

        triggers: vec![(
            "run".to_string(),
            TaskTriggerInput {
                input_id: base.script_input_id.clone(),
                name: "Run a script".to_string(),
                description: None,
                periodic: None,
            },
        )]
        .into_iter()
        .collect::<FxHashMap<_, _>>(),
    };

    let state_machine_task_id = base
        .user
        .client
        .new_task(&state_machine_task)
        .await
        .expect("bootstrap: writing state_machine_task")
        .task_id;

    (state_machine_task_id, state_machine_task)
}

async fn bootstrap_script_task(base: &BootstrappedData) -> (TaskId, TaskInput) {
    let script = r##"
        let context = Ergo.getContext() ?? { value: 1 };
        let url = Ergo.getPayload().url;
        context.value++;
        Ergo.setContext(context);
        Ergo.runAction('send', {
            url,
            payload: context
        });
        "##
    .to_string();

    let script_task = TaskInput {
        name: "script task".to_string(),
        description: None,
        alias: None,
        enabled: true,
        state: Some(TaskState::Js(TaskJsState {
            context: String::new(),
        })),
        source: serde_json::Value::Null,
        compiled: TaskConfig::Js(TaskJsConfig {
            map: String::new(),
            script,
            timeout: None,
        }),
        triggers: vec![(
            "request_url".to_string(),
            TaskTriggerInput {
                name: "request".to_string(),
                description: None,
                input_id: base.url_input_id.clone(),
                periodic: None,
            },
        )]
        .into_iter()
        .collect(),
        actions: vec![(
            "send".to_string(),
            TaskActionInput {
                name: "Send a request".to_string(),
                action_id: base.http_action_id.clone(),
                account_id: None,
                action_template: Some(vec![(
                    "method".to_string(),
                    serde_json::Value::String("POST".to_string()),
                )]),
            },
        )]
        .into_iter()
        .collect(),
    };

    let script_task_id = base
        .user
        .client
        .new_task(&script_task)
        .await
        .expect("bootstrap: writing script_task")
        .task_id;

    (script_task_id, script_task)
}

async fn bootstrap_dataflow_task(base: &BootstrappedData) -> (TaskId, TaskInput) {
    let dataflow_nodes = vec![
        DataFlowNode {
            name: "input_url".to_string(),
            func: DataFlowNodeFunction::Trigger(DataFlowTrigger {
                local_id: "request_url".to_string(),
            }),
        },
        DataFlowNode {
            name: "input_doc".to_string(),
            func: DataFlowNodeFunction::Trigger(DataFlowTrigger {
                local_id: "doc_id".to_string(),
            }),
        },
        DataFlowNode {
            name: "fetch_value".to_string(),
            func: DataFlowNodeFunction::Js(DataFlowJs {
                code:
                    r##"(base_url && doc_id) ? fetch(base_url.url + doc_id.value).then((r) => r.json()) : null"##
                        .into(),
                format: JsCodeFormat::Expression,
            }),
        },
        DataFlowNode {
            name: "action".to_string(),
            func: DataFlowNodeFunction::Action(DataFlowAction {
                action_id: "send".into(),
                payload_code: DataFlowJs {
                    code: r##"fetch_result ? { url: fetch_result.url, payload: { value: "abc" } } : null"##.to_string(),
                    format: JsCodeFormat::Expression,
                },
            }),
        },
    ];

    let edges = [
        ("input_url", "fetch_value", "base_url"),
        ("input_doc", "fetch_value", "doc_id"),
        ("fetch_value", "action", "fetch_result"),
    ];
    let edges = edge_indexes_from_names(&dataflow_nodes, &edges).expect("building edges");

    let dataflow_config = TaskConfig::DataFlow(
        DataFlowConfig::new(dataflow_nodes, edges).expect("building dataflow config"),
    );
    let dataflow_state = dataflow_config.default_state();

    let dataflow_task = TaskInput {
        name: "dataflow task".to_string(),
        description: None,
        compiled: dataflow_config,
        state: None,
        source: serde_json::Value::Null,
        alias: None,
        enabled: true,
        triggers: vec![
            (
                "request_url".to_string(),
                TaskTriggerInput {
                    name: "Set the base URL".to_string(),
                    description: None,
                    input_id: base.url_input_id.clone(),
                    periodic: None,
                },
            ),
            (
                "doc_id".to_string(),
                TaskTriggerInput {
                    name: "Set the doc id".to_string(),
                    description: None,
                    input_id: base.string_input_id.clone(),
                    periodic: None,
                },
            ),
        ]
        .into_iter()
        .collect(),
        actions: vec![(
            "send".to_string(),
            TaskActionInput {
                name: "Send a request".to_string(),
                action_id: base.http_action_id.clone(),
                account_id: None,
                action_template: Some(vec![(
                    "method".to_string(),
                    serde_json::Value::String("POST".to_string()),
                )]),
            },
        )]
        .into_iter()
        .collect(),
    };

    let dataflow_task_id = base
        .user
        .client
        .new_task(&dataflow_task)
        .await
        .expect("bootstrap: writing script_task")
        .task_id;

    (dataflow_task_id, dataflow_task)
}

async fn wait_for_actionless_task_to_finish(
    user: &TestUser,
    log_id: &Uuid,
) -> Result<Vec<InputsLogEntry>, anyhow::Error> {
    let mut logs = user.client.get_recent_logs().await?;
    if !logs.is_empty() {
        event!(Level::INFO, ?logs);
    }

    let mut num_checks = 0;
    while !logs
        .iter()
        .find(|l| &l.inputs_log_id == log_id)
        .map(|l| l.input_status == InputStatus::Success || l.input_status == InputStatus::Error)
        .unwrap_or(false)
    {
        tokio::time::sleep(Duration::from_secs(1)).await;
        logs = user.client.get_recent_logs().await?;
        if !logs.is_empty() {
            event!(Level::INFO, ?logs);
        }
        num_checks += 1;
        if num_checks > 5 {
            panic!("Timed out waiting for logs, last saw {:?}", logs)
        }
    }

    Ok(logs)
}

async fn wait_for_task_to_finish(
    user: &TestUser,
    log_id: &Uuid,
) -> Result<Vec<InputsLogEntry>, anyhow::Error> {
    let mut logs = user.client.get_recent_logs().await?;
    event!(Level::INFO, ?logs);

    let mut num_checks = 0;
    while !logs
        .iter()
        .find(|l| &l.inputs_log_id == log_id)
        .and_then(|i| i.actions.0.get(0))
        .map(|a| a.status == ActionStatus::Error || a.status == ActionStatus::Success)
        .unwrap_or(false)
    {
        tokio::time::sleep(Duration::from_secs(1)).await;
        logs = user.client.get_recent_logs().await?;
        num_checks += 1;
        if num_checks > 5 {
            panic!("Timed out waiting for logs, last saw {:?}", logs)
        }
    }

    Ok(logs)
}

#[actix_rt::test]
async fn state_machine_task() {
    run_app_test(|app| async move {
        let base = bootstrap(&app).await?;
        let (state_machine_task_id, _) = bootstrap_state_machine_task(&base).await;
        let BootstrappedData { user, .. } = base;

        let script = r##"Ergo.setResult({ value: 5 })"##;

        let log_id = user
            .client
            .run_task_trigger("run_script", "run", json!({ "script": script }))
            .await?
            .log_id;

        let logs = wait_for_task_to_finish(&user, &log_id).await?;

        println!("{:?}", logs);
        assert_eq!(logs[0].inputs_log_id, log_id);
        assert_eq!(logs[0].input_status, InputStatus::Success);
        assert_eq!(logs[0].task_trigger_name, "Run a script");
        assert_eq!(logs[0].task_trigger_local_id, "run");
        assert_eq!(logs[0].task_id, state_machine_task_id);
        assert_eq!(logs[0].actions.len(), 1);
        assert_eq!(
            logs[0].actions[0].result,
            json!({ "output": { "result": {"value": 5 }, "console": [] } }),
            "executor result"
        );
        assert_eq!(logs[0].actions[0].status, ActionStatus::Success);

        Ok(())
    })
    .await
}

#[actix_rt::test]
async fn postprocess_script() {
    run_app_test(|app| async move {
        let base = bootstrap(&app).await?;
        let (state_machine_task_id, _) = bootstrap_state_machine_task(&base).await;
        let BootstrappedData {
            user,
            script_action,
            mut script_action_payload,
            ..
        } = base;

        script_action_payload.postprocess_script =
            Some(r##"return { ...output, pp: output.result.value + 10 };"##.to_string());
        app.admin_user
            .client
            .put_action(&script_action.action_id, &script_action_payload)
            .await
            .unwrap();

        let script = r##"Ergo.setResult({ value: 5 })"##;

        let log_id = user
            .client
            .run_task_trigger("run_script", "run", json!({ "script": script }))
            .await?
            .log_id;

        let logs = wait_for_task_to_finish(&user, &log_id).await?;

        println!("{:?}", logs);
        assert_eq!(logs[0].inputs_log_id, log_id);
        assert_eq!(logs[0].input_status, InputStatus::Success);
        assert_eq!(logs[0].task_trigger_name, "Run a script");
        assert_eq!(logs[0].task_trigger_local_id, "run");
        assert_eq!(logs[0].task_id, state_machine_task_id);
        assert_eq!(logs[0].actions.len(), 1);
        assert_eq!(logs[0].actions[0].status, ActionStatus::Success);
        assert_eq!(
            logs[0].actions[0].result,
            json!({ "output": { "pp": 15, "result": {"value": 5 }, "console": [] } }),
            "executor result"
        );

        Ok(())
    })
    .await
}

#[actix_rt::test]
async fn postprocess_script_returns_nothing() {
    run_app_test(|app| async move {
        let base = bootstrap(&app).await?;
        let (state_machine_task_id, _) = bootstrap_state_machine_task(&base).await;
        let BootstrappedData {
            user,
            script_action,
            mut script_action_payload,
            ..
        } = base;

        // Normally there would be more actual checking here.
        script_action_payload.postprocess_script = Some("return".to_string());
        app.admin_user
            .client
            .put_action(&script_action.action_id, &script_action_payload)
            .await
            .expect("writing action");

        let script = r##"Ergo.setResult({ value: 5 })"##;

        let log_id = user
            .client
            .run_task_trigger("run_script", "run", json!({ "script": script }))
            .await
            .expect("running task trigger")
            .log_id;

        let logs = wait_for_task_to_finish(&user, &log_id).await?;

        println!("{:?}", logs);
        assert_eq!(logs[0].inputs_log_id, log_id);
        assert_eq!(logs[0].input_status, InputStatus::Success);
        assert_eq!(logs[0].task_trigger_name, "Run a script");
        assert_eq!(logs[0].task_trigger_local_id, "run");
        assert_eq!(logs[0].task_id, state_machine_task_id);
        assert_eq!(logs[0].actions.len(), 1);
        assert_eq!(logs[0].actions[0].status, ActionStatus::Success);
        assert_eq!(
            logs[0].actions[0].result,
            json!({ "output": { "result": {"value": 5 }, "console": [] } }),
            "executor result"
        );

        Ok(())
    })
    .await
}

#[actix_rt::test]
async fn script_task() {
    run_app_test(|app| async move {
        let base = bootstrap(&app).await.expect("bootstrapping app");
        let (script_task_id, _) = bootstrap_script_task(&base).await;
        let BootstrappedData { user, .. } = base;
        let mock_server = MockServer::start().await;

        let expected_body = json!({ "value": 2 });
        Mock::given(method("POST"))
            .and(path("/a_url"))
            .and(body_json(expected_body))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!("the response")))
            .mount(&mock_server)
            .await;

        let url = format!("{}/a_url", mock_server.uri());

        println!("Running task first time");
        let log_id = user
            .client
            .run_task_trigger(
                script_task_id.to_string().as_str(),
                "request_url",
                json!({ "url": url }),
            )
            .await
            .expect("running task trigger")
            .log_id;

        let logs = wait_for_task_to_finish(&user, &log_id).await?;

        println!("{:?}", logs);
        assert_eq!(logs[0].inputs_log_id, log_id);
        assert_eq!(logs[0].input_status, InputStatus::Success);
        assert_eq!(logs[0].task_trigger_name, "request");
        assert_eq!(logs[0].task_trigger_local_id, "request_url");
        assert_eq!(logs[0].task_id, script_task_id);
        assert_eq!(logs[0].actions.len(), 1);
        assert_eq!(logs[0].actions[0].status, ActionStatus::Success);
        assert_eq!(
            logs[0].actions[0].result,
            json!({ "output": { "response": "the response", "status": 200 } }),
            "executor result"
        );

        mock_server.verify().await;

        println!("Running task second time");

        mock_server.reset().await;
        let expected_body = json!({ "value": 3 });
        Mock::given(method("POST"))
            .and(path("/a_url"))
            .and(body_json(expected_body))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!("the response")))
            .mount(&mock_server)
            .await;

        let log_id = user
            .client
            .run_task_trigger(
                script_task_id.to_string().as_str(),
                "request_url",
                json!({ "url": url }),
            )
            .await
            .expect("running task trigger")
            .log_id;

        let logs = wait_for_task_to_finish(&user, &log_id).await?;

        println!("{:?}", logs);
        assert_eq!(logs[0].inputs_log_id, log_id);
        assert_eq!(logs[0].input_status, InputStatus::Success);
        assert_eq!(logs[0].task_trigger_name, "request");
        assert_eq!(logs[0].task_trigger_local_id, "request_url");
        assert_eq!(logs[0].task_id, script_task_id);
        assert_eq!(logs[0].actions.len(), 1);
        assert_eq!(logs[0].actions[0].status, ActionStatus::Success);
        assert_eq!(
            logs[0].actions[0].result,
            json!({ "output": { "response": "the response", "status": 200 } }),
            "executor result"
        );

        mock_server.verify().await;

        Ok(())
    })
    .await
}

#[actix_rt::test]
async fn dataflow_task() {
    run_app_test(|app| async move {
        let base = bootstrap(&app).await.expect("bootstrapping app");
        let (dataflow_task_id, _) = bootstrap_dataflow_task(&base).await;
        let BootstrappedData { user, .. } = base;
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/a_url/test_doc"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "url": format!("{}/final_url", mock_server.uri()) })),
            )
            .mount(&mock_server)
            .await;

        let expected_body = json!({ "value": "abc" });
        Mock::given(method("POST"))
            .and(path("/final_url"))
            .and(body_json(expected_body))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "value": "ok!" })))
            .mount(&mock_server)
            .await;

        println!("Running task to set doc id");
        let log_id = user
            .client
            .run_task_trigger(
                dataflow_task_id.to_string().as_str(),
                "doc_id",
                json!({ "value": "test_doc" }),
            )
            .await
            .expect("running task trigger")
            .log_id;

        let logs = wait_for_actionless_task_to_finish(&user, &log_id).await?;

        println!("{:?}", logs);
        assert_eq!(logs[0].inputs_log_id, log_id);
        assert_eq!(logs[0].input_status, InputStatus::Success);
        assert_eq!(logs[0].task_trigger_name, "Set the doc id");
        assert_eq!(logs[0].task_trigger_local_id, "doc_id");
        assert_eq!(logs[0].task_id, dataflow_task_id);
        assert_eq!(logs[0].actions.len(), 0);
        mock_server.verify().await;

        println!("Running task to set fetch base url");

        let url = format!("{}/a_url/", mock_server.uri());
        let log_id = user
            .client
            .run_task_trigger(
                dataflow_task_id.to_string().as_str(),
                "request_url",
                json!({ "url": url }),
            )
            .await
            .expect("running task trigger")
            .log_id;

        let logs = wait_for_task_to_finish(&user, &log_id).await?;

        println!("{:?}", logs);
        assert_eq!(logs[0].inputs_log_id, log_id);
        assert_eq!(logs[0].input_status, InputStatus::Success);
        assert_eq!(logs[0].task_trigger_name, "Set the base URL");
        assert_eq!(logs[0].task_trigger_local_id, "request_url");
        assert_eq!(logs[0].task_id, dataflow_task_id);
        assert_eq!(logs[0].actions.len(), 1);
        assert_eq!(logs[0].actions[0].status, ActionStatus::Success);
        assert_eq!(
            logs[0].actions[0].result,
            json!({ "output": { "response": { "value": "ok!" }, "status": 200 } }),
            "action executor result"
        );

        mock_server.verify().await;

        Ok(())
    })
    .await
}
