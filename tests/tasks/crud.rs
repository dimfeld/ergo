use crate::common::{run_app_test, TestApp, TestUser};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use ergo::tasks::{
    handlers::{NewTaskResult, TaskDescription, TaskInput},
    state_machine,
};
use futures::future::join_all;
use fxhash::FxHashMap;
use serde_json::json;
use uuid::Uuid;

struct BootstrappedData {
    org_id: Uuid,
    user1: TestUser,
    user2: TestUser,
    user1_tasks: Vec<(NewTaskResult, TaskInput)>,
    user2_task: (NewTaskResult, TaskInput),
    reference_time: DateTime<Utc>,
}

async fn bootstrap_data(app: &TestApp) -> Result<BootstrappedData> {
    let org_id = app.add_org("user org").await?;
    let user1 = app.add_user(&org_id, "User 1").await?;
    let user2 = app.add_user(&org_id, "User 2").await?;

    let user1_tasks = vec![
        TaskInput {
            name: "task 1".to_string(),
            description: Some("task 1 description".to_string()),
            enabled: true,
            state_machine_config: state_machine::StateMachineConfig::new(),
            state_machine_states: state_machine::StateMachineStates::new(),
            actions: vec![].into_iter().collect::<FxHashMap<_, _>>(),
            triggers: vec![].into_iter().collect::<FxHashMap<_, _>>(),
        },
        TaskInput {
            name: "task 2".to_string(),
            description: Some("a task 2 description".to_string()),
            enabled: true,
            state_machine_config: state_machine::StateMachineConfig::new(),
            state_machine_states: state_machine::StateMachineStates::new(),
            actions: vec![].into_iter().collect::<FxHashMap<_, _>>(),
            triggers: vec![].into_iter().collect::<FxHashMap<_, _>>(),
        },
        TaskInput {
            name: "task 3".to_string(),
            description: None,
            enabled: false,
            state_machine_config: state_machine::StateMachineConfig::new(),
            state_machine_states: state_machine::StateMachineStates::new(),
            actions: vec![].into_iter().collect::<FxHashMap<_, _>>(),
            triggers: vec![].into_iter().collect::<FxHashMap<_, _>>(),
        },
    ];

    let user2_task = TaskInput {
        name: "user2 task".to_string(),
        description: None,
        enabled: true,
        state_machine_config: state_machine::StateMachineConfig::new(),
        state_machine_states: state_machine::StateMachineStates::new(),
        actions: vec![].into_iter().collect::<FxHashMap<_, _>>(),
        triggers: vec![].into_iter().collect::<FxHashMap<_, _>>(),
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
                        id: String::new(),
                        name: task.name.clone(),
                        description: task.description.clone(),
                        enabled: task.enabled,
                        created: reference_time.clone(),
                        modified: reference_time.clone(),
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
            assert_eq!(task.description, expected.description);
            assert_eq!(task.enabled, expected.enabled);
            assert!(task.created > reference_time);
            assert!(task.modified > reference_time);
            assert!(task.id.len() > 0);
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
                task.state_machine_config,
                json!([]),
                "Task {}: state machine config should match",
                i
            );
            assert_eq!(
                task.state_machine_states,
                json!([]),
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
            assert_eq!(
                task.triggers,
                json!([]),
                "Task {}: triggers list should match",
                i
            );
            assert_eq!(
                task.actions,
                json!([]),
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
            .get_task(id.as_str())
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

#[test]
#[ignore]
fn delete_task() {}

#[test]
#[ignore]
fn post_new_task() {}

#[test]
#[ignore]
fn put_existing_task() {}

#[test]
#[ignore]
fn put_new_task() {}

#[test]
#[ignore]
fn put_task_without_write_permission() {}

#[test]
#[ignore]
fn list_inputs() {}

#[test]
#[ignore]
fn new_input() {}

#[test]
#[ignore]
fn update_input() {}

#[test]
#[ignore]
fn delete_input() {}

#[test]
#[ignore]
fn list_actions() {}

#[test]
#[ignore]
fn new_action() {}

#[test]
#[ignore]
fn update_action() {}

#[test]
#[ignore]
fn delete_action() {}
