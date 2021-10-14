use ergo_api::routes::tasks::{NewTaskResult, TaskInput};
use ergo_database::object_id::OrgId;

use crate::{
    common::{TestApp, TestUser},
    tasks::{simple_state_machine, simple_task_actions},
};

use super::{
    bootstrap_inputs_and_actions, simple_task_triggers, BootstrappedActions, BootstrappedInputs,
};

struct BootstrappedData {
    org_id: OrgId,
    user: TestUser,
    task: (NewTaskResult, TaskInput),
    inputs: BootstrappedInputs,
    actions: BootstrappedActions,
}

async fn bootstrap_data(app: &TestApp) -> BootstrappedData {
    let org_id = app.add_org("user org").await.expect("creating org");
    let user = app
        .add_user(&org_id, "User 1")
        .await
        .expect("creating user");

    let (inputs, actions) = bootstrap_inputs_and_actions(app).await;
    let (config, state) = simple_state_machine();

    let task_input = TaskInput {
        name: "task".to_string(),
        alias: None,
        description: None,
        enabled: true,
        compiled: config,
        state,
        source: serde_json::Value::Null,
        actions: simple_task_actions(&actions),
        triggers: simple_task_triggers(&inputs),
    };

    let task = user
        .client
        .new_task(&task_input)
        .await
        .expect("bootsrap: creating task");

    BootstrappedData {
        org_id,
        user,
        task: (task, task_input),
        inputs,
        actions,
    }
}

#[actix_rt::test]
#[ignore]
async fn new_task_with_periodic_triggers() {}

#[actix_rt::test]
#[ignore]
async fn alter_periodic_trigger_payload() {}

#[actix_rt::test]
#[ignore]
async fn alter_periodic_trigger_schedule() {}

#[actix_rt::test]
#[ignore]
async fn add_second_periodic_trigger() {}

#[actix_rt::test]
#[ignore]
async fn delete_periodic_trigger() {}

#[actix_rt::test]
#[ignore]
async fn disable_periodic_trigger() {}

#[actix_rt::test]
#[ignore]
async fn disable_task() {}
