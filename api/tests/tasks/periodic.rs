use chrono::{DateTime, Datelike, Duration, DurationRound, Timelike, Utc};
use ergo_api::routes::tasks::{NewTaskResult, TaskInput};
use ergo_database::{object_id::OrgId, RedisPool};
use ergo_tasks::{inputs::queue::InputQueue, PeriodicSchedule, PeriodicTaskTriggerInput};
use ergo_test::wait_for;
use serde_json::json;

use crate::{
    common::{run_app_test, TestApp, TestUser},
    tasks::{simple_state_machine, simple_task_actions},
};

use super::{
    bootstrap_inputs_and_actions, simple_task_triggers, BootstrappedActions, BootstrappedInputs,
};

fn cron_for_date(date: &DateTime<Utc>) -> PeriodicSchedule {
    let cron = format!(
        "0 {} {} {} {} * *",
        date.minute(),
        date.hour(),
        date.day(),
        date.month()
    );

    PeriodicSchedule::Cron(cron)
}

struct BootstrappedData {
    org_id: OrgId,
    user: TestUser,
    task: (NewTaskResult, TaskInput),
    inputs: BootstrappedInputs,
    input_queue: InputQueue,
    actions: BootstrappedActions,
    schedule_date: DateTime<Utc>,
}

async fn bootstrap_data(app: &TestApp) -> BootstrappedData {
    let org_id = app.add_org("user org").await.expect("creating org");
    let user = app
        .add_user(&org_id, "User 1")
        .await
        .expect("creating user");

    let (inputs, actions) = bootstrap_inputs_and_actions(app).await;
    let (config, state) = simple_state_machine();

    let mut triggers = simple_task_triggers(&inputs);

    let now = Utc::now().duration_trunc(Duration::minutes(1)).unwrap();
    let schedule_date = now + Duration::days(2);

    triggers.get_mut("run_it").unwrap().periodic = Some(vec![PeriodicTaskTriggerInput {
        name: None,
        enabled: true,
        payload: json!({ "url": "https://abc.com/" }),
        schedule: cron_for_date(&schedule_date),
    }]);

    let task_input = TaskInput {
        name: "task".to_string(),
        alias: None,
        description: None,
        enabled: true,
        compiled: config,
        state,
        source: serde_json::Value::Null,
        actions: simple_task_actions(&actions),
        triggers,
    };

    let task = user
        .client
        .new_task(&task_input)
        .await
        .expect("bootstrap: creating task");
    let redis_pool = RedisPool::new(app.redis_url.clone(), Some(app.redis_key_prefix.clone()))
        .expect("Creating Redis pool");
    let input_queue = InputQueue::new(redis_pool);

    BootstrappedData {
        org_id,
        user,
        task: (task, task_input),
        input_queue,
        inputs,
        actions,
        schedule_date,
    }
}

#[actix_rt::test]
async fn new_task_with_periodic_triggers() {
    run_app_test(|app| async move {
        let BootstrappedData {
            input_queue,
            schedule_date,
            ..
        } = bootstrap_data(&app).await;

        // The task was already set up by bootstrap, so just check the result.
        let scheduled = wait_for(|| async {
            let values = input_queue
                .list_scheduled()
                .await
                .expect("Retrieving scheduled jobs");

            Some(values).filter(|v| !v.is_empty())
        })
        .await
        .expect("Queue was not populated with trigger");

        assert!(!scheduled.is_empty(), "trigger is scheduled");
        assert_eq!(
            scheduled[0].1, schedule_date,
            "trigger is scheduled at the expected time"
        );

        Ok(())
    })
    .await;
}

#[actix_rt::test]
async fn alter_periodic_trigger_payload() {
    run_app_test(|app| async move {
        let BootstrappedData {
            input_queue,
            schedule_date,
            task: (task, mut task_input),
            user,
            ..
        } = bootstrap_data(&app).await;
        //
        // The task was already set up by bootstrap, so just check the result.
        let scheduled = wait_for(|| async {
            let values = input_queue
                .list_scheduled()
                .await
                .expect("Retrieving scheduled jobs");

            Some(values).filter(|v| !v.is_empty())
        })
        .await
        .expect("Queue was not populated with trigger");

        let queue_task_id = scheduled[0].0.as_str();

        task_input
            .triggers
            .get_mut("run_it")
            .unwrap()
            .periodic
            .as_mut()
            .unwrap()[0]
            .payload = json!({ "url": "https://newurl.com" });
        user.client
            .put_task(&task.task_id, &task_input)
            .await
            .expect("writing task with altered payload");

        wait_for(|| async {
            let job_details = input_queue
                .job_info(queue_task_id)
                .await
                .expect("retrieving job payload");

            job_details.filter(|j| {
                println!("job {:?}", j);
                let new_payload: serde_json::Value =
                    serde_json::from_slice(&j.payload).expect("deserializing payload");
                println!("Got new payload: {:?}", new_payload);
                new_payload == json!({"url": "https://newurl.com" })
            })
        })
        .await
        .expect("Waiting for payload to update");

        let scheduled = input_queue
            .list_scheduled()
            .await
            .expect("Getting scheduled jobs");
        println!("Scheduled jobs: {:?}", scheduled);
        assert_eq!(
            scheduled[0].1, schedule_date,
            "trigger is still scheduled at the same time"
        );

        Ok(())
    })
    .await;
}

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

#[actix_rt::test]
#[ignore]
async fn invalid_payload() {}

#[actix_rt::test]
#[ignore]
async fn invalid_schedule() {}
