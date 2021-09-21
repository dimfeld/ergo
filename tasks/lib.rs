pub mod actions;
mod error;
pub mod inputs;
#[cfg(not(target_family = "wasm"))]
pub mod queue_drain_runner;
pub mod scripting;
pub mod state_machine;

pub use error::*;
#[cfg(not(target_family = "wasm"))]
pub use native::*;

use fxhash::{FxHashMap, FxHashSet};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
#[serde(tag = "type", content = "data")]
pub enum TaskConfig {
    StateMachine(state_machine::StateMachineConfig),
    // JS(scripting::TaskJsConfig),
}

impl TaskConfig {
    pub fn validate(
        &self,
        actions: &impl StringKeyContainer,
        inputs: &impl StringKeyContainer,
    ) -> Result<(), TaskValidateErrors> {
        let errors = match self {
            Self::StateMachine(machines) => {
                let mut errors = Vec::new();
                for m in machines {
                    errors.extend_from_slice(m.validate(actions, inputs).as_slice());
                }
                errors
            }
        };

        if errors.is_empty() {
            Ok(())
        } else {
            Err(TaskValidateErrors(errors))
        }
    }
}

pub trait StringKeyContainer {
    fn has(&self, key: &String) -> bool;
}

impl StringKeyContainer for FxHashSet<String> {
    fn has(&self, key: &String) -> bool {
        self.contains(key)
    }
}

impl<V> StringKeyContainer for FxHashMap<String, V> {
    fn has(&self, key: &String) -> bool {
        self.contains_key(key)
    }
}

#[cfg(not(target_family = "wasm"))]
mod native {
    use crate::{
        actions::{execute::execute, ActionStatus},
        error::*,
        inputs::InputStatus,
        state_machine::{ActionInvocations, StateMachineStates, StateMachineWithData},
        TaskConfig,
    };
    use chrono::{DateTime, Utc};
    use ergo_database::{
        object_id::{InputId, OrgId, TaskId, TaskTemplateId, TaskTriggerId},
        sql_insert_parameters,
        transaction::serializable,
        PostgresPool,
    };
    use ergo_notifications::{Notification, NotificationManager, NotifyEvent};
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};
    use sqlx::{types::Json, FromRow};
    use tracing::{event, instrument, Level};
    use uuid::Uuid;

    #[derive(Clone, Debug, Deserialize, JsonSchema, Serialize, PartialEq, Eq)]
    #[serde(tag = "type", content = "data")]
    pub enum TaskState {
        StateMachine(StateMachineStates),
        // JS(TaskJsState)
    }

    #[derive(Serialize, Deserialize, FromRow)]
    pub struct Task {
        pub task_id: TaskId,
        pub org_id: Uuid,
        pub name: String,
        pub description: Option<String>,
        pub enabled: bool,
        pub task_template_id: uuid::Uuid,
        pub task_template_version: TaskTemplateId,
        pub config: Json<TaskConfig>,
        pub state: Json<TaskState>,
        pub created: DateTime<Utc>,
        pub modified: DateTime<Utc>,
    }

    const GET_TASK_QUERY: &'static str = r##"SELECT task_id as "task_id: TaskId",
              org_id, name,
              description, enabled,
              state_machine_config as "state_machine_config: Json<state_machine::StateMachineConfig>",
              state_machine_states as "state_machine_states: Json<state_machine::StateMachineStates>",
              created, modified
            FROM tasks WHERE task_id = $1"##;

    impl Task {
        /// Apply an input to a task.
        /// Instead of acting on an existing task instance, this loads the task
        /// and applies the input inside a serializable transaction, to ensure that
        /// the applied input doesn't have a race condition with any other concurrent
        /// inputs to the same task.
        #[instrument(skip(pool, notifications))]
        pub async fn apply_input(
            pool: &PostgresPool,
            notifications: Option<NotificationManager>,
            task_id: TaskId,
            input_id: InputId,
            task_trigger_id: TaskTriggerId,
            input_arrival_id: uuid::Uuid,
            payload: serde_json::Value,
            immediate_actions: bool,
        ) -> Result<(), Error> {
            let immediate_data = if immediate_actions {
                Some(notifications.clone())
            } else {
                None
            };

            let mut conn = pool.acquire().await?;
            let result = serializable(&mut conn, 5, move |tx| {
            let payload = payload.clone();
            let input_arrival_id = input_arrival_id.clone();
            let notifications = notifications.clone();
            let task_id = task_id.clone();
            let task_trigger_id = task_trigger_id.clone();
            Box::pin(async move {
                #[derive(Debug, FromRow)]
                struct TaskInputData {
                    task_trigger_local_id: String,
                    config: Json<TaskConfig>,
                    state: Json<TaskState>,
                    org_id: OrgId,
                    task_name: String,
                    task_trigger_name: String,
                }

                let task = sqlx::query_as!(TaskInputData,
                        r##"SELECT task_trigger_local_id,
                        compiled as "config!: Json<TaskConfig>",
                        state as "state!: Json<TaskState>",
                        tasks.org_id as "org_id: OrgId",
                        tasks.name as task_name,
                        tt.name as task_trigger_name
                        FROM tasks
                        JOIN task_templates USING (task_template_id, task_template_version)
                        JOIN task_triggers tt ON tt.task_id=$1 AND task_trigger_id=$2
                        WHERE tasks.task_id=$1"##,
                        &task_id.0,
                        &task_trigger_id.0
                    )
                    .fetch_optional(&mut *tx)
                    .await?;

                let task = task.ok_or(Error::NotFound)?;

                let TaskInputData {
                    task_trigger_local_id, config, state, org_id, task_name, task_trigger_name
                } = task;

                let (new_data, actions, changed) = match (config.0, state.0) {
                    (TaskConfig::StateMachine(machine), TaskState::StateMachine(state)) => {
                      let num_machines = machine.len();
                      let mut new_data = StateMachineStates::with_capacity(num_machines);
                      let mut actions = ActionInvocations::new();
                      let mut changed = false;
                      for (idx, (machine, state)) in machine
                        .into_iter()
                        .zip(state.into_iter())
                        .enumerate() {
                              let mut m = StateMachineWithData::new(task_id.clone(), idx, machine, state);
                              let this_actions = m
                                  .apply_trigger(
                                      &task_trigger_local_id,
                                      &Some(input_arrival_id),
                                      Some(&payload),
                                  ).await
                                  .map_err(Error::from)?;

                              let (data, this_changed) = m.take();
                              new_data.push(data);
                              actions.extend(this_actions.into_iter());
                              changed = changed || this_changed;
                            }

                      (TaskState::StateMachine(new_data), actions, changed)
                  }
                };


                if changed {
                    event!(Level::INFO, state=?new_data, "New state");
                    sqlx::query!(
                        r##"UPDATE tasks
                        SET state = $1::jsonb
                        WHERE task_id = $2;
                        "##,
                        serde_json::value::to_value(&new_data)?,
                        *task_id,
                    )
                    .execute(&mut *tx)
                    .await?;
                }

                if !actions.is_empty() {
                    event!(Level::INFO, ?actions, "Enqueueing actions");
                    let q = format!(
                        "INSERT INTO actions_log (task_id, task_action_local_id, actions_log_id, inputs_log_id, payload, status)
                        VALUES
                        {}
                        ",
                        sql_insert_parameters::<6>(actions.len())
                    );

                    let mut log_query = sqlx::query(&q);

                    for action in &actions {
                        log_query = log_query
                            .bind(&action.task_id)
                            .bind(&action.task_action_local_id)
                            .bind(action.actions_log_id)
                            .bind(action.input_arrival_id)
                            .bind(&action.payload)
                            .bind(ActionStatus::Pending);
                    }

                    log_query.fetch_all(&mut *tx).await?;

                    if !immediate_actions {
                        let q = format!(
                            r##"INSERT INTO action_queue
                            (task_id, task_action_local_id, actions_log_id, input_arrival_id, payload)
                            VALUES
                            {}
                            "##,
                            sql_insert_parameters::<5>(actions.len())
                        );

                        let mut query = sqlx::query(&q);
                        for action in &actions {
                            query = query
                                .bind(&action.task_id)
                                .bind(&action.task_action_local_id)
                                .bind(action.actions_log_id)
                                .bind(action.input_arrival_id)
                                .bind(&action.payload);
                        }

                        query.execute(&mut *tx).await?;
                    }
                }

                if let Some(notifications) = notifications {
                    let input_notification = Notification{
                        event: NotifyEvent::InputProcessed,
                        payload: Some(payload),
                        task_id,
                        task_name,
                        local_id: task_trigger_local_id,
                        local_object_name: task_trigger_name,
                        local_object_id: Some(task_trigger_id.into()),
                        error: None,
                        log_id: Some(input_arrival_id),
                    };
                    notifications.notify(tx, &org_id, input_notification).await?;
                }
                Ok::<_, Error>(actions)
            })
        })
        .await;

            let (log_error, status, retval) = match result {
                Ok(actions) => {
                    if let Some(notifications) = immediate_data {
                        for action in actions {
                            let pool = pool.clone();
                            let notifications = notifications.clone();
                            tokio::task::spawn(async move {
                                execute(&pool, notifications.as_ref(), &action).await
                            });
                        }
                    }
                    (None, InputStatus::Success, Ok(()))
                }
                Err(e) => {
                    event!(Level::ERROR, err=?e, "Error applying input");
                    (
                        Some(
                            serde_json::json!({ "msg": e.to_string(), "info": format!("{:?}", e) }),
                        ),
                        InputStatus::Error,
                        Err(e),
                    )
                }
            };

            event!(Level::INFO, %input_arrival_id, ?status, ?log_error, "Updating input status");
            sqlx::query!(
                "UPDATE inputs_log SET status=$2, error=$3, updated=now() WHERE inputs_log_id=$1",
                input_arrival_id,
                status as _,
                log_error
            )
            .execute(pool)
            .await?;

            retval
        }
    }

    #[derive(Serialize, Deserialize)]
    pub struct TaskTrigger {
        pub task_trigger_id: TaskTriggerId,
        pub task_id: TaskId,
        pub input_id: InputId,
        pub last_payload: Option<Box<serde_json::value::RawValue>>,
    }
}
