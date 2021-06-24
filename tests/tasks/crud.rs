use crate::common::run_app_test;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use ergo::tasks::{
    handlers::{TaskDescription, TaskInput},
    state_machine,
};
use futures::future::join_all;
use fxhash::FxHashMap;
use serde_json::json;

#[actix_rt::test]
async fn list_tasks() {
    run_app_test(|app| async move {
        let org_id = app.add_org("user org").await?;
        let user1 = app.add_user(&org_id, "User 1").await?;
        let user2 = app.add_user(&org_id, "User 2").await?;

        let new_tasks = vec![
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
        join_all(
            new_tasks
                .iter()
                .map(|task| user1.client.new_task(task))
                .chain(vec![user2.client.new_task(&user2_task)])
                .collect::<Vec<_>>(),
        )
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

        let expected_tasks = new_tasks
            .iter()
            .map(|task| {
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

#[test]
#[ignore]
fn get_task() {}

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
