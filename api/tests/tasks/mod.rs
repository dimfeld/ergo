use std::borrow::Cow;

use ergo_api::routes::{
    actions::ActionPayload,
    inputs::InputPayload,
    tasks::{TaskActionInput, TaskTriggerInput},
};
use ergo_tasks::{
    actions::{execute::ScriptOrTemplate, template::TemplateField, Action},
    inputs::Input,
    state_machine::{StateDefinition, StateMachine, StateMachineData},
    TaskConfig, TaskState,
};
use futures::future::join;
use fxhash::FxHashMap;
use serde_json::json;
use smallvec::smallvec;

use crate::common::TestApp;

mod crud;
mod execution;
mod periodic;

pub struct BootstrappedInputs {
    pub url: Input,
}

pub struct BootstrappedActions {
    pub echo: Action,
}

pub async fn bootstrap_inputs_and_actions(
    app: &TestApp,
) -> (BootstrappedInputs, BootstrappedActions) {
    let url_input_payload = InputPayload {
        input_category_id: None,
        name: "URL".to_string(),
        description: None,
        payload_schema: json!({
          "$schema": "http://json-schema.org/draft-07/schema",
          "$id": "http://ergo.dev/inputs/url.json",
          "type": "object",
          "required": [
              "url"
          ],
          "properties": {
              "url": {
                  "type": "string",
                  "format": "url"
              }
          },
          "additionalProperties": true
        }),
    };

    let echo_action_payload = ActionPayload {
        action_category_id: app.base_action_category.clone(),
        name: "Echo".to_string(),
        postprocess_script: None,
        description: Some("Echo the input".to_string()),
        executor_id: "raw_command".to_string(),
        executor_template: ScriptOrTemplate::Template(vec![
            ("command".to_string(), json!("/bin/echo")),
            ("args".to_string(), json!(["{{text}}"])),
        ]),
        template_fields: vec![TemplateField {
            name: Cow::from("text"),
            format: ergo_tasks::actions::template::TemplateFieldFormat::string_without_default(),
            optional: false,
            description: None,
        }]
        .into(),
        account_required: false,
        account_types: vec![],
        timeout: None,
    };

    let (url_input, echo_action) = join(
        app.admin_user.client.new_input(&url_input_payload),
        app.admin_user.client.new_action(&echo_action_payload),
    )
    .await;

    let url_input = url_input.expect("Creating url input");
    let echo_action = echo_action.expect("Creating echo action");

    (
        BootstrappedInputs { url: url_input },
        BootstrappedActions { echo: echo_action },
    )
}

pub fn simple_state_machine() -> (TaskConfig, TaskState) {
    let machine = StateMachine {
        name: "a sample machine".to_string(),
        description: None,
        initial: "start".to_string(),
        on: smallvec![],
        states: std::array::IntoIter::new([(
            "initial".to_string(),
            StateDefinition {
                on: smallvec![],
                description: None,
            },
        )])
        .collect::<FxHashMap<_, _>>(),
    };

    let state = StateMachineData {
        state: "initial".to_string(),
        context: json!(null),
    };

    (
        TaskConfig::StateMachine(smallvec![machine]),
        TaskState::StateMachine(smallvec![state]),
    )
}

pub fn simple_task_actions(actions: &BootstrappedActions) -> FxHashMap<String, TaskActionInput> {
    vec![
        (
            "run".to_string(),
            TaskActionInput {
                name: "Run the action".to_string(),
                action_id: actions.echo.action_id.clone(),
                account_id: None,
                action_template: None,
            },
        ),
        (
            "ask".to_string(),
            TaskActionInput {
                name: "Ask a question".to_string(),
                action_id: actions.echo.action_id.clone(),
                account_id: None,
                action_template: None,
            },
        ),
    ]
    .into_iter()
    .collect::<FxHashMap<_, _>>()
}

pub fn simple_task_triggers(inputs: &BootstrappedInputs) -> FxHashMap<String, TaskTriggerInput> {
    vec![
        (
            "run_it".to_string(),
            TaskTriggerInput {
                name: "run it".to_string(),
                description: Some("Run the task and do something".to_string()),
                input_id: inputs.url.input_id.clone(),
                periodic: None,
            },
        ),
        (
            "prepare".to_string(),
            TaskTriggerInput {
                name: "Get ready to do something".to_string(),
                description: None,
                input_id: inputs.url.input_id.clone(),
                periodic: None,
            },
        ),
    ]
    .into_iter()
    .collect::<FxHashMap<_, _>>()
}
