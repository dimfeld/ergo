use std::{borrow::Cow, time::Duration};

use anyhow::Result;
use ergo_api::routes::{
    actions::ActionPayload,
    inputs::InputPayload,
    tasks::{TaskActionInput, TaskInput, TaskTriggerInput},
};
use ergo_database::object_id::{ActionId, InputId, OrgId, TaskId};
use ergo_tasks::{
    actions::{
        execute::ScriptOrTemplate,
        template::{TemplateField, TemplateFieldFormat},
        Action, ActionStatus,
    },
    inputs::{Input, InputStatus},
    state_machine::{
        ActionInvokeDef, ActionPayloadBuilder, EventHandler, StateDefinition, StateMachine,
        StateMachineData,
    },
    TaskConfig, TaskState,
};
use fxhash::FxHashMap;
use serde_json::json;
use smallvec::smallvec;

use crate::common::{run_app_test, TestApp, TestUser};

struct BootstrappedData {
    org: OrgId,
    user: TestUser,
    script_input: Input,
    script_input_payload: InputPayload,
    script_action: Action,
    script_action_payload: ActionPayload,
    task: TaskInput,
    task_id: TaskId,
}

async fn bootstrap(app: &TestApp) -> Result<BootstrappedData> {
    let org = app.add_org("user org").await?;
    let user = app.add_user(&org, "user 1").await?;
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
            format: TemplateFieldFormat::String,
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

    let task = TaskInput {
        name: "Run script".to_string(),
        description: None,
        alias: Some("run_script".to_string()),
        enabled: true,
        state: TaskState::StateMachine(smallvec![StateMachineData {
            state: "initial".to_string(),
            context: json!(null)
        }]),

        source: json!(null),
        compiled: TaskConfig::StateMachine(smallvec![StateMachine {
            name: "run_action".to_string(),
            description: None,
            initial: "initial".to_string(),
            on: smallvec![],
            states: std::array::IntoIter::new([(
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
            )])
            .collect::<FxHashMap<_, _>>()
        }]),

        actions: vec![(
            "run".to_string(),
            TaskActionInput {
                name: "Run the script".to_string(),
                action_id: script_action_id.clone(),
                account_id: None,
                action_template: None,
            },
        )]
        .into_iter()
        .collect::<FxHashMap<_, _>>(),

        triggers: vec![(
            "run".to_string(),
            TaskTriggerInput {
                input_id: script_input_id.clone(),
                name: "Run a script".to_string(),
                description: None,
            },
        )]
        .into_iter()
        .collect::<FxHashMap<_, _>>(),
    };

    let task_id = user
        .client
        .new_task(&task)
        .await
        .expect("bootstrap: writing task")
        .task_id;

    Ok(BootstrappedData {
        org,
        user,
        script_input,
        script_input_payload,
        script_action,
        script_action_payload,
        task,
        task_id,
    })
}

#[actix_rt::test]
async fn script_task() {
    run_app_test(|app| async move {
        let BootstrappedData { user, task_id, .. } = bootstrap(&app).await?;

        let script = r##"Ergo.setResult({ value: 5 })"##;

        let log_id = user
            .client
            .run_task_trigger("run_script", "run", json!({ "script": script }))
            .await?
            .log_id;

        let mut logs = user.client.get_recent_logs().await?;
        println!("{:?}", logs);

        let mut num_checks = 0;
        while logs
            .get(0)
            .and_then(|i| i.actions.0.get(0))
            .map(|a| a.status == ActionStatus::Error || a.status == ActionStatus::Success)
            .unwrap_or(false)
            == false
        {
            tokio::time::sleep(Duration::from_secs(1)).await;
            logs = user.client.get_recent_logs().await?;
            num_checks = num_checks + 1;
            if num_checks > 30 {
                panic!("Timed out waiting for logs, last saw {:?}", logs)
            }
        }

        println!("{:?}", logs);
        assert_eq!(logs[0].inputs_log_id, log_id);
        assert_eq!(logs[0].input_status, InputStatus::Success);
        assert_eq!(logs[0].task_trigger_name, "Run a script");
        assert_eq!(logs[0].task_trigger_local_id, "run");
        assert_eq!(logs[0].task_id, task_id);
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
        let BootstrappedData {
            user,
            script_action,
            mut script_action_payload,
            task_id,
            ..
        } = bootstrap(&app).await?;

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

        let mut logs = user.client.get_recent_logs().await?;
        println!("{:?}", logs);

        let mut num_checks = 0;
        while logs
            .get(0)
            .and_then(|i| i.actions.0.get(0))
            .map(|a| a.status == ActionStatus::Error || a.status == ActionStatus::Success)
            .unwrap_or(false)
            == false
        {
            tokio::time::sleep(Duration::from_secs(1)).await;
            logs = user.client.get_recent_logs().await?;
            num_checks = num_checks + 1;
            if num_checks > 30 {
                panic!("Timed out waiting for logs, last saw {:?}", logs)
            }
        }

        println!("{:?}", logs);
        assert_eq!(logs[0].inputs_log_id, log_id);
        assert_eq!(logs[0].input_status, InputStatus::Success);
        assert_eq!(logs[0].task_trigger_name, "Run a script");
        assert_eq!(logs[0].task_trigger_local_id, "run");
        assert_eq!(logs[0].task_id, task_id);
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
        let BootstrappedData {
            user,
            script_action,
            mut script_action_payload,
            task_id,
            ..
        } = bootstrap(&app).await.expect("bootstrapping app");

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

        let mut logs = user.client.get_recent_logs().await?;
        println!("{:?}", logs);

        let mut num_checks = 0;
        while logs
            .get(0)
            .and_then(|i| i.actions.0.get(0))
            .map(|a| a.status == ActionStatus::Error || a.status == ActionStatus::Success)
            .unwrap_or(false)
            == false
        {
            tokio::time::sleep(Duration::from_secs(1)).await;
            logs = user.client.get_recent_logs().await?;
            num_checks = num_checks + 1;
            if num_checks > 30 {
                panic!("Timed out waiting for logs, last saw {:?}", logs)
            }
        }

        println!("{:?}", logs);
        assert_eq!(logs[0].inputs_log_id, log_id);
        assert_eq!(logs[0].input_status, InputStatus::Success);
        assert_eq!(logs[0].task_trigger_name, "Run a script");
        assert_eq!(logs[0].task_trigger_local_id, "run");
        assert_eq!(logs[0].task_id, task_id);
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
