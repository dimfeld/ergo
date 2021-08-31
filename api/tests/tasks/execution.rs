use std::{borrow::Cow, time::Duration};

use anyhow::Result;
use ergo_api::tasks::{
    actions::{
        execute::ScriptOrTemplate,
        handlers::ActionPayload,
        template::{TemplateField, TemplateFieldFormat},
        ActionStatus,
    },
    handlers::{TaskActionInput, TaskInput, TaskTriggerInput},
    inputs::handlers::InputPayload,
    state_machine::{
        ActionInvokeDef, ActionPayloadBuilder, EventHandler, StateDefinition, StateMachine,
        StateMachineData,
    },
    TaskConfig, TaskState,
};
use ergo_database::object_id::{ActionId, InputId, OrgId, TaskId};
use fxhash::FxHashMap;
use serde_json::json;
use smallvec::smallvec;

use crate::common::{run_app_test, TestApp, TestUser};

struct BootstrappedData {
    org: OrgId,
    user: TestUser,
    script_input: InputPayload,
    script_action: ActionPayload,
    task: TaskInput,
    task_id: TaskId,
}

async fn bootstrap(app: &TestApp) -> Result<BootstrappedData> {
    let org = app.add_org("user org").await?;
    let user = app.add_user(&org, "user 1").await?;
    let script_input = InputPayload {
        name: "run script".to_string(),
        description: None,
        input_id: Some(InputId::new()),
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

    app.admin_user
        .client
        .put_input(script_input.input_id.as_ref().unwrap(), &script_input)
        .await?;

    let script_action = ActionPayload {
        action_id: Some(ActionId::new()),
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
        }],
        account_required: false,
        account_types: None,
        postprocess_script: None,
    };

    app.admin_user
        .client
        .put_action(script_action.action_id.as_ref().unwrap(), &script_action)
        .await?;

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
                action_id: script_action.action_id.clone().unwrap(),
                account_id: None,
                action_template: None,
            },
        )]
        .into_iter()
        .collect::<FxHashMap<_, _>>(),

        triggers: vec![(
            "run".to_string(),
            TaskTriggerInput {
                input_id: script_input.input_id.clone().unwrap(),
                name: "Run a script".to_string(),
                description: None,
            },
        )]
        .into_iter()
        .collect::<FxHashMap<_, _>>(),
    };

    let task_id = user.client.new_task(&task).await?.task_id;

    Ok(BootstrappedData {
        org,
        user,
        script_input,
        script_action,
        task,
        task_id,
    })
}

#[actix_rt::test]
async fn script_task() {
    run_app_test(|app| async move {
        let BootstrappedData {
            org,
            user,
            script_input,
            script_action,
            task,
            task_id,
        } = bootstrap(&app).await?;

        let script = r##"result = { value: 5 }"##;

        let log_id = user
            .client
            .run_task_trigger("run_script", "run", json!({ "script": script }))
            .await?
            .log_id;

        let mut logs = user.client.get_recent_logs().await?;
        println!("{:?}", logs);

        while logs
            .get(0)
            .and_then(|i| i.actions.0.get(0))
            .map(|a| a.status == ActionStatus::Error || a.status == ActionStatus::Success)
            .unwrap_or(false)
            == false
        {
            tokio::time::sleep(Duration::from_secs(1)).await;
            logs = user.client.get_recent_logs().await?;
        }

        println!("{:?}", logs);
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
async fn postprocess_script() {
    run_app_test(|app| async move {
        let BootstrappedData {
            org,
            user,
            script_input,
            mut script_action,
            task,
            task_id,
        } = bootstrap(&app).await?;

        script_action.postprocess_script =
            Some(r##"return { ...output, pp: output.result.value + 10 };"##.to_string());
        app.admin_user
            .client
            .put_action(&script_action.action_id.as_ref().unwrap(), &script_action)
            .await
            .unwrap();

        let script = r##"result = { value: 5 }"##;

        let log_id = user
            .client
            .run_task_trigger("run_script", "run", json!({ "script": script }))
            .await?
            .log_id;

        let mut logs = user.client.get_recent_logs().await?;
        println!("{:?}", logs);

        while logs
            .get(0)
            .and_then(|i| i.actions.0.get(0))
            .map(|a| a.status == ActionStatus::Error || a.status == ActionStatus::Success)
            .unwrap_or(false)
            == false
        {
            tokio::time::sleep(Duration::from_secs(1)).await;
            logs = user.client.get_recent_logs().await?;
        }

        println!("{:?}", logs);
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
