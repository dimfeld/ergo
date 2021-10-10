use std::{borrow::Cow, collections::HashSet};

use crate::{
    common::{run_app_test, TestApp, TestUser},
    compare_hashmaps,
};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use ergo_api::routes::{
    actions::ActionPayload,
    inputs::InputPayload,
    tasks::{NewTaskResult, TaskActionInput, TaskDescription, TaskInput, TaskTriggerInput},
};
use ergo_database::object_id::{OrgId, TaskId};
use ergo_tasks::{
    actions::{execute::ScriptOrTemplate, template::TemplateField, Action},
    inputs::Input,
    state_machine::{StateDefinition, StateMachine, StateMachineData},
    TaskConfig, TaskState,
};
use futures::future::{join, join_all};
use fxhash::FxHashMap;
use serde_json::json;
use smallvec::smallvec;

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

struct BootstrappedInputs {
    url: Input,
}

struct BootstrappedActions {
    echo: Action,
}

struct BootstrappedData {
    org_id: OrgId,
    user1: TestUser,
    user2: TestUser,
    user1_tasks: Vec<(NewTaskResult, TaskInput)>,
    user2_task: (NewTaskResult, TaskInput),
    reference_time: DateTime<Utc>,
    inputs: BootstrappedInputs,
    actions: BootstrappedActions,
}

async fn bootstrap_data(app: &TestApp) -> Result<BootstrappedData> {
    let org_id = app.add_org("user org").await?;
    let user1 = app.add_user(&org_id, "User 1").await?;
    let user2 = app.add_user(&org_id, "User 2").await?;

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
            format: ergo_tasks::actions::template::TemplateFieldFormat::String,
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

    let (machine, states) = simple_state_machine();

    let test_actions = vec![
        (
            "run".to_string(),
            TaskActionInput {
                name: "Run the action".to_string(),
                action_id: echo_action.action_id.clone(),
                account_id: None,
                action_template: None,
            },
        ),
        (
            "ask".to_string(),
            TaskActionInput {
                name: "Ask a question".to_string(),
                action_id: echo_action.action_id.clone(),
                account_id: None,
                action_template: None,
            },
        ),
    ]
    .into_iter()
    .collect::<FxHashMap<_, _>>();

    let test_triggers = vec![
        (
            "run_it".to_string(),
            TaskTriggerInput {
                name: "run it".to_string(),
                description: Some("Run the task and do something".to_string()),
                input_id: url_input.input_id.clone(),
                periodic: None,
            },
        ),
        (
            "prepare".to_string(),
            TaskTriggerInput {
                name: "Get ready to do something".to_string(),
                description: None,
                input_id: url_input.input_id.clone(),
                periodic: None,
            },
        ),
    ]
    .into_iter()
    .collect::<FxHashMap<_, _>>();

    let user1_tasks = vec![
        TaskInput {
            name: "task 1".to_string(),
            alias: None,
            description: Some("task 1 description".to_string()),
            enabled: true,
            compiled: machine.clone(),
            source: serde_json::Value::Null,
            state: states.clone(),
            actions: test_actions.clone(),
            triggers: test_triggers.clone(),
        },
        TaskInput {
            name: "task 2".to_string(),
            alias: Some("task_2".to_string()),
            description: Some("a task 2 description".to_string()),
            enabled: true,
            compiled: machine.clone(),
            source: serde_json::Value::Null,
            state: states.clone(),
            actions: test_actions.clone(),
            triggers: test_triggers.clone(),
        },
        TaskInput {
            name: "task 3".to_string(),
            alias: None,
            description: None,
            enabled: false,
            compiled: machine.clone(),
            source: serde_json::Value::Null,
            state: states.clone(),
            actions: test_actions.clone(),
            triggers: test_triggers.clone(),
        },
    ];

    let user2_task = TaskInput {
        name: "user2 task".to_string(),
        alias: None,
        description: None,
        enabled: true,
        compiled: machine.clone(),
        source: serde_json::Value::Null,
        state: states.clone(),
        actions: test_actions.clone(),
        triggers: test_triggers.clone(),
    };

    let reference_time = Utc::now();

    let user1_task_results = join_all(
        user1_tasks
            .into_iter()
            .map(|task| async { user1.client.new_task(&task).await.map(|r| (r, task)) })
            .collect::<Vec<_>>(),
    )
    .await
    .into_iter()
    .collect::<Result<Vec<_>, _>>()?;

    let user2_task_result = user2.client.new_task(&user2_task).await?;

    Ok(BootstrappedData {
        org_id,
        user1,
        user2,
        user1_tasks: user1_task_results,
        user2_task: (user2_task_result, user2_task),
        reference_time,
        inputs: BootstrappedInputs { url: url_input },
        actions: BootstrappedActions { echo: echo_action },
    })
}

#[actix_rt::test]
async fn list_tasks() {
    run_app_test(|app| async move {
        let BootstrappedData {
            user1,
            user1_tasks,
            reference_time,
            ..
        } = bootstrap_data(&app).await?;

        let expected_tasks = user1_tasks
            .iter()
            .map(|(_, task)| {
                (
                    task.name.clone(),
                    TaskDescription {
                        task_id: TaskId::new(),
                        alias: task.alias.clone(),
                        name: task.name.clone(),
                        description: task.description.clone(),
                        enabled: task.enabled,
                        created: reference_time.clone(),
                        modified: reference_time.clone(),
                        last_triggered: None,
                        successes: 0,
                        failures: 0,
                        stats_since: Utc::now() - chrono::Duration::days(7),
                    },
                )
            })
            .collect::<FxHashMap<_, _>>();

        // We should see user 1's tasks, but not user 2's tasks.
        let task_list = user1.client.list_tasks().await?;
        for task in &task_list {
            let expected = expected_tasks
                .get(task.name.as_str())
                .ok_or_else(|| anyhow!("API returned unexpected task {}", task.name.as_str()))?;

            assert_eq!(task.name, expected.name);
            assert_eq!(task.alias, expected.alias);
            assert_eq!(task.description, expected.description);
            assert_eq!(task.enabled, expected.enabled);
            assert_eq!(task.last_triggered, expected.last_triggered);
            assert_eq!(task.successes, expected.successes);
            assert_eq!(task.failures, expected.failures);
            assert!(task.stats_since - expected.stats_since < chrono::Duration::minutes(1));
            assert!(task.created > reference_time);
            assert!(task.modified > reference_time);
            assert!(task.task_id.to_string().starts_with("tsk"));
        }

        assert_eq!(task_list.len(), 3, "Expecting three tasks");

        Ok(())
    })
    .await
}

#[actix_rt::test]
async fn get_task() {
    run_app_test(|app| async move {
        let BootstrappedData {
            user1,
            user1_tasks,
            reference_time,
            ..
        } = bootstrap_data(&app).await?;

        for (i, (result, input)) in user1_tasks.into_iter().enumerate() {
            let task = user1.client.get_task(&result.task_id).await?;

            assert_eq!(
                result.task_id, task.task_id,
                "Task {}: Task ID should match the requested one",
                i
            );

            assert_eq!(input.name, task.name, "Task {}: Task name should match", i);
            assert_eq!(
                task.description, input.description,
                "Task {}: Task description should match",
                i
            );
            assert_eq!(
                task.enabled, input.enabled,
                "Task {}: enabled should match",
                i
            );
            assert_eq!(
                task.compiled.0, input.compiled,
                "Task {}: state machine config should match",
                i
            );
            assert_eq!(
                task.state.0, input.state,
                "Task {}: state machine states should match",
                i
            );
            assert!(
                task.created >= reference_time,
                "Task {}: created time should be after reference time",
                i
            );
            assert!(
                task.modified >= reference_time,
                "Task {}: modified time should be after reference time",
                i
            );

            compare_hashmaps!(
                input.triggers,
                task.triggers.0,
                "Task {}: triggers list should match",
                i
            );
            compare_hashmaps!(
                input.actions,
                task.actions.0,
                "Task {}: actions list should match",
                i
            );
        }

        Ok(())
    })
    .await
}

#[actix_rt::test]
async fn get_task_without_permission() {
    run_app_test(|app| async move {
        let BootstrappedData {
            user1,
            user2,
            user2_task,
            ..
        } = bootstrap_data(&app).await?;

        // First make sure that the task is set up properly.
        let id = user2_task.0.task_id;
        user2
            .client
            .get_task(&id)
            .await
            .expect("User 2 should be able to see its own task");

        user1
            .client
            .get_task(&id)
            .await
            .expect_err("User 1 should not be able to read other user's task");

        Ok(())
    })
    .await
}

#[actix_rt::test]
async fn delete_task() {
    run_app_test(|app| async move {
        let BootstrappedData {
            user1,
            user2,
            user1_tasks,
            ..
        } = bootstrap_data(&app).await?;

        assert_eq!(
            user1.client.list_tasks().await?.len(),
            3,
            "3 tasks to start"
        );

        let deleted_task = &user1_tasks[2].0.task_id;

        // First try to delete it from a user that doesn't have permissions.
        user2
            .client
            .delete_task(deleted_task)
            .await
            .expect_err("User 2 should fail to delete user 1's task");

        assert_eq!(
            user1.client.list_tasks().await?.len(),
            3,
            "User 2's attempt to delete the task should not work"
        );

        user1.client.delete_task(deleted_task).await?;

        let remaining_tasks = user1.client.list_tasks().await?;
        let remaining_task_ids = remaining_tasks
            .into_iter()
            .map(|t| t.task_id)
            .collect::<HashSet<_>>();

        assert!(
            remaining_task_ids.get(&user1_tasks[0].0.task_id).is_some(),
            "task 0 remains"
        );
        assert!(
            remaining_task_ids.get(&user1_tasks[1].0.task_id).is_some(),
            "task 1 remains"
        );
        assert!(
            remaining_task_ids.get(&user1_tasks[2].0.task_id).is_none(),
            "task 2 was deleted"
        );

        Ok(())
    })
    .await
}

#[actix_rt::test]
async fn put_existing_task() {
    run_app_test(|app| async move {
        let BootstrappedData {
            user1,
            user2,
            user1_tasks,
            user2_task,
            ..
        } = bootstrap_data(&app).await?;

        let task_id = &user1_tasks[0].0.task_id;

        user2
            .client
            .put_task(task_id, &user2_task.1)
            .await
            .expect_err("User 2 can not update user 1's task");

        let task = user1.client.get_task(task_id).await?;
        assert_eq!(
            task.name, user1_tasks[0].1.name,
            "Task should not be changed by disallowed update"
        );

        let (config, state) = simple_state_machine();

        let updated = TaskInput {
            name: "new name".to_string(),
            alias: None,
            description: Some("a new description".to_string()),
            enabled: false,
            compiled: config.clone(),
            source: serde_json::Value::Null,
            state: state.clone(),
            actions: vec![].into_iter().collect::<FxHashMap<_, _>>(),
            triggers: vec![].into_iter().collect::<FxHashMap<_, _>>(),
        };

        user1
            .client
            .put_task(task_id, &updated)
            .await
            .expect("Updating task");

        let result = user1
            .client
            .get_task(task_id)
            .await
            .expect("Retrieving updated task");
        assert_eq!(result.name, updated.name);
        assert_eq!(result.description, updated.description);
        assert_eq!(result.enabled, updated.enabled);

        Ok(())
    })
    .await
}

#[actix_rt::test]
async fn update_task_triggers() {
    run_app_test(|app| async move {
        let BootstrappedData {
            user1,
            user1_tasks,
            inputs,
            ..
        } = bootstrap_data(&app).await?;

        let mut task = user1_tasks[0].1.clone();
        task.triggers.insert(
            "do_it".to_string(),
            TaskTriggerInput {
                name: "Do the thing".to_string(),
                description: None,
                input_id: inputs.url.input_id.clone(),
                periodic: None,
            },
        );

        user1
            .client
            .put_task(&user1_tasks[0].0.task_id, &task)
            .await
            .expect("Adding trigger");
        let added_trigger_result = user1
            .client
            .get_task(&user1_tasks[0].0.task_id)
            .await
            .expect("Retrieving task with added trigger");
        compare_hashmaps!(
            task.triggers,
            added_trigger_result.triggers.0,
            "trigger was added successfully"
        );

        task.triggers.remove("prepare");
        user1
            .client
            .put_task(&user1_tasks[0].0.task_id, &task)
            .await
            .expect("Removing trigger");
        let removed_trigger_result = user1
            .client
            .get_task(&user1_tasks[0].0.task_id)
            .await
            .expect("Retrieving task with removed trigger");
        compare_hashmaps!(
            task.triggers,
            removed_trigger_result.triggers.0,
            "trigger was added successfully"
        );

        // Update a trigger
        task.triggers.insert(
            "do_it".to_string(),
            TaskTriggerInput {
                name: "Do another thing".to_string(),
                description: Some("A description".to_string()),
                input_id: inputs.url.input_id.clone(),
                periodic: None,
            },
        );

        user1
            .client
            .put_task(&user1_tasks[0].0.task_id, &task)
            .await
            .expect("Modifying trigger");
        let updated_trigger_result = user1
            .client
            .get_task(&user1_tasks[0].0.task_id)
            .await
            .expect("Retrieving task with updated trigger");
        compare_hashmaps!(
            task.triggers,
            updated_trigger_result.triggers.0,
            "trigger was added successfully"
        );

        // And now try all operations at once.
        let mut task2 = user1_tasks[1].1.clone();
        task2.triggers.remove("prepare");
        task2.triggers.insert(
            "do_it".to_string(),
            TaskTriggerInput {
                name: "Do another thing".to_string(),
                description: Some("A description".to_string()),
                input_id: inputs.url.input_id.clone(),
                periodic: None,
            },
        );
        task2.triggers.insert(
            "run_it".to_string(),
            TaskTriggerInput {
                name: "changed run it".to_string(),
                description: Some("this is another change".to_string()),
                input_id: inputs.url.input_id.clone(),
                periodic: None,
            },
        );
        task2.triggers.insert(
            "see_it".to_string(),
            TaskTriggerInput {
                name: "see a thing".to_string(),
                description: None,
                input_id: inputs.url.input_id.clone(),
                periodic: None,
            },
        );

        user1
            .client
            .put_task(&user1_tasks[1].0.task_id, &task2)
            .await
            .expect("Multiple trigger updates");
        let updated_trigger_result = user1
            .client
            .get_task(&user1_tasks[1].0.task_id)
            .await
            .expect("Retrieving task with multiple updates");
        compare_hashmaps!(
            task2.triggers,
            updated_trigger_result.triggers.0,
            "triggers were updated"
        );

        Ok(())
    })
    .await
}

#[actix_rt::test]
async fn update_task_actions() {
    run_app_test(|app| async move {
        let BootstrappedData {
            user1,
            user1_tasks,
            actions,
            ..
        } = bootstrap_data(&app).await?;

        let mut task = user1_tasks[0].1.clone();
        task.actions.insert(
            "do_it".to_string(),
            TaskActionInput {
                name: "Do a thing".to_string(),
                action_id: actions.echo.action_id.clone(),
                account_id: None,
                action_template: None,
            },
        );
        user1
            .client
            .put_task(&user1_tasks[0].0.task_id, &task)
            .await
            .expect("Adding action");
        let added_action_result = user1
            .client
            .get_task(&user1_tasks[0].0.task_id)
            .await
            .expect("Retrieving task with added action");
        compare_hashmaps!(
            task.actions,
            added_action_result.actions.0,
            "action was added successfully"
        );

        task.actions.remove("run_it");
        user1
            .client
            .put_task(&user1_tasks[0].0.task_id, &task)
            .await
            .expect("Removing action");
        let removed_action_result = user1
            .client
            .get_task(&user1_tasks[0].0.task_id)
            .await
            .expect("Retrieving task with removed action");
        compare_hashmaps!(
            task.actions,
            removed_action_result.actions.0,
            "action was removed successfully"
        );

        // Update an action
        task.actions.insert(
            "ask".to_string(),
            TaskActionInput {
                name: "Ask it".to_string(),
                action_id: actions.echo.action_id.clone(),
                account_id: None,
                action_template: None,
            },
        );
        user1
            .client
            .put_task(&user1_tasks[0].0.task_id, &task)
            .await
            .expect("Modifying action");
        let modified_action_result = user1
            .client
            .get_task(&user1_tasks[0].0.task_id)
            .await
            .expect("Retrieving task with modified action");
        compare_hashmaps!(
            task.actions,
            modified_action_result.actions.0,
            "action was modified successfully"
        );

        // Try multiple changes at once
        let mut task2 = user1_tasks[1].1.clone();
        task2.actions.remove("run");
        task2.actions.insert(
            "ask".to_string(),
            TaskActionInput {
                name: "Ask it".to_string(),
                action_id: actions.echo.action_id.clone(),
                account_id: None,
                action_template: None,
            },
        );
        task2.actions.insert(
            "do_it".to_string(),
            TaskActionInput {
                name: "Do a thing".to_string(),
                action_id: actions.echo.action_id.clone(),
                account_id: None,
                action_template: None,
            },
        );
        task2.actions.insert(
            "add_another".to_string(),
            TaskActionInput {
                name: "Do another thing".to_string(),
                action_id: actions.echo.action_id.clone(),
                account_id: None,
                action_template: None,
            },
        );

        user1
            .client
            .put_task(&user1_tasks[1].0.task_id, &task2)
            .await
            .expect("Modifying action");
        let result = user1
            .client
            .get_task(&user1_tasks[1].0.task_id)
            .await
            .expect("Retrieving task with modified action");
        compare_hashmaps!(
            task2.actions,
            result.actions.0,
            "actions were modified successfully"
        );

        Ok(())
    })
    .await
}

#[actix_rt::test]
async fn list_inputs() {
    run_app_test(|app| async move {
        let BootstrappedData { user1, inputs, .. } =
            bootstrap_data(&app).await.expect("Bootstrapping");
        let input_list = user1
            .client
            .list_inputs()
            .await
            .expect("Listing inputs")
            .into_iter()
            .map(|i| (i.input_id.clone(), i))
            .collect::<FxHashMap<_, _>>();
        let expected_inputs = std::array::IntoIter::new([inputs.url.clone()])
            .map(|i| (i.input_id.clone(), i))
            .collect::<FxHashMap<_, _>>();
        assert_eq!(input_list, expected_inputs, "Inputs match expected list");

        Ok(())
    })
    .await
}

#[test]
#[ignore]
fn new_input() {}

#[test]
#[ignore]
fn update_input() {}

#[test]
#[ignore]
fn delete_input() {}

#[actix_rt::test]
async fn list_actions() {
    run_app_test(|app| async move {
        let BootstrappedData { user1, actions, .. } =
            bootstrap_data(&app).await.expect("Bootstrapping");

        let action_list = user1
            .client
            .list_actions()
            .await
            .expect("Listing actions")
            .into_iter()
            .map(|i| (i.action_id.clone(), i))
            .collect::<FxHashMap<_, _>>();

        let expected_actions = std::array::IntoIter::new([actions.echo.clone()])
            .map(|i| (i.action_id.clone(), i))
            .collect::<FxHashMap<_, _>>();

        assert_eq!(expected_actions, action_list, "actions match expected list");

        Ok(())
    })
    .await
}

#[test]
#[ignore]
fn new_action() {}

#[test]
#[ignore]
fn update_action() {}

#[test]
#[ignore]
fn delete_action() {}
