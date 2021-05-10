pub mod actions;
pub mod handlers;
pub mod inputs;
pub mod queue_drain_runner;
mod state_machine;

use std::{error::Error as StdError, pin::Pin, sync::Arc};

use smallvec::SmallVec;
pub use state_machine::StateMachineError;

use crate::{
    database::{sql_insert_parameters, transaction::serializable, PostgresPool},
    error::Error,
    tasks::{actions::ActionStatus, inputs::InputStatus},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{types::Json, Executor, FromRow, Postgres, Row, Transaction};
use uuid::Uuid;

use self::{
    actions::TaskAction,
    state_machine::{
        ActionInvocations, StateMachineData, StateMachineStates, StateMachineWithData,
    },
};

#[derive(Serialize, Deserialize, FromRow)]
pub struct Task {
    pub task_id: i64,
    pub external_task_id: String,
    pub org_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub state_machine_config: Json<state_machine::StateMachineConfig>,
    pub state_machine_states: Json<state_machine::StateMachineStates>,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
}

const GET_TASK_QUERY: &'static str = r##"SELECT task_id, external_task_id, org_id, name,
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
    pub async fn apply_input(
        pool: &PostgresPool,
        task_id: i64,
        input_id: i64,
        task_trigger_id: i64,
        input_arrival_id: uuid::Uuid,
        payload: serde_json::Value,
    ) -> Result<(), Error> {
        let result = serializable(pool, 5, move |tx| {
            let payload = payload.clone();
            let input_arrival_id = input_arrival_id.clone();
            Box::pin(async move {
                #[derive(Debug, FromRow)]
                struct TaskInputData {
                    task_trigger_local_id: String,
                    state_machine_config: Json<state_machine::StateMachineConfig>,
                    state_machine_states: Json<state_machine::StateMachineStates>,
                }

                let task = sqlx::query_as!(TaskInputData,
                        r##"SELECT task_trigger_local_id,
                        state_machine_config as "state_machine_config: Json<state_machine::StateMachineConfig>" ,
                        state_machine_states as "state_machine_states: Json<state_machine::StateMachineStates>"
                        FROM tasks
                        JOIN task_triggers tt ON tt.task_id=$1 AND task_trigger_id=$2"##,
                        task_id,
                        task_trigger_id
                    )
                    .fetch_optional(&mut *tx)
                    .await?;

                let task = task.ok_or(Error::NotFound)?;

                let TaskInputData {
                    task_trigger_local_id, state_machine_config, state_machine_states
                } = task;

                let num_machines = state_machine_config.len();
                let (new_data, actions, changed) = state_machine_config
                    .0
                    .into_iter()
                    .zip(state_machine_states.0.into_iter())
                    .enumerate()
                    .try_fold(
                        (
                            StateMachineStates::with_capacity(num_machines),
                            ActionInvocations::new(),
                            false,
                        ),
                        |mut acc, (idx, (machine, state))| {
                            let mut m = StateMachineWithData::new(task_id, idx, machine, state);
                            let actions = m
                                .apply_trigger(
                                    &task_trigger_local_id,
                                    &Some(input_arrival_id),
                                    Some(&payload),
                                )
                                .map_err(Error::from)?;

                            let (data, changed) = m.take();
                            acc.0.push(data);
                            acc.1.extend(actions.into_iter());
                            acc.2 = acc.2 || changed;
                            Ok(acc) as Result<(StateMachineStates, ActionInvocations, bool), Error>
                        },
                    )?;

                if changed {
                    sqlx::query!(
                        r##"UPDATE tasks
                        SET state_machine_states = $1::jsonb
                        WHERE task_id = $2;
                        "##,
                        serde_json::value::to_value(&new_data)?,
                        task_id,
                    )
                    .execute(&mut *tx)
                    .await?;
                }

                if !actions.is_empty() {
                    let q = format!(
                        "INSERT INTO actions_log (task_id, task_action_local_id, actions_log_id, inputs_log_id, payload, status)
                        VALUES
                        {}
                        ",
                        sql_insert_parameters::<5>(actions.len())
                    );

                    let mut log_query = sqlx::query(&q);

                    for action in &actions {
                        log_query = log_query
                            .bind(action.task_id)
                            .bind(&action.task_action_local_id)
                            .bind(action.actions_log_id)
                            .bind(action.input_arrival_id)
                            .bind(&action.payload)
                            .bind(ActionStatus::Pending);
                    }

                    let action_log_ids = log_query.fetch_all(&mut *tx).await?;

                    let q = format!(
                        r##"INSERT INTO action_queue
                        (task_id, task_action_id, actions_log_id, input_arrival_id, payload)
                        VALUES
                        {}
                        "##,
                        sql_insert_parameters::<5>(actions.len())
                    );

                    let mut query = sqlx::query(&q);
                    for action in actions {
                        query = query
                            .bind(action.task_id)
                            .bind(action.task_action_local_id)
                            .bind(action.actions_log_id)
                            .bind(action.input_arrival_id)
                            .bind(action.payload);
                    }

                    query.execute(&mut *tx).await?;
                }
                Ok::<(), Error>(())
            })
        })
        .await;

        let (log_error, status) = match &result {
            Ok(_) => (None, InputStatus::Success),
            Err(e) => (
                Some(serde_json::json!({ "msg": e.to_string(), "info": format!("{:?}", e) })),
                InputStatus::Error,
            ),
        };

        sqlx::query!(
            "UPDATE inputs_log SET status=$2, error=$3, updated=now() WHERE inputs_log_id=$1",
            input_arrival_id,
            status as _,
            log_error
        )
        .execute(pool)
        .await?;

        result
    }
}

#[derive(Serialize, Deserialize)]
pub struct TaskTrigger {
    pub task_trigger_id: i64,
    pub task_id: i64,
    pub input_id: i64,
    pub last_payload: Option<Box<serde_json::value::RawValue>>,
}
