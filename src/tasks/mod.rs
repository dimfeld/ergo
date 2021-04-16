pub mod actions;
pub mod executor;
pub mod handlers;
pub mod inputs;
pub mod queue_drain_runner;
mod state_machine;

use std::pin::Pin;

use smallvec::SmallVec;
pub use state_machine::StateMachineError;

use crate::{
    database::{sql_insert_parameters, transaction::serializable, PostgresPool},
    error::Error,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{types::Json, Executor, FromRow, Postgres, Transaction};
use uuid::Uuid;

use self::state_machine::{
    ActionInvocations, StateMachineData, StateMachineStates, StateMachineWithData,
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
    pub async fn from_db(pool: &PostgresPool, task_id: i64) -> Result<Option<Task>, Error> {
        sqlx::query_as::<Postgres, Task>(GET_TASK_QUERY)
            .bind(task_id)
            .fetch_optional(pool)
            .await
            .map_err(Error::from)
    }

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
        payload: serde_json::Value,
    ) -> Result<(), Error> {
        serializable(pool, 5, move |tx| {
            let payload = payload.clone();
            Box::pin(async move {
                let task = sqlx::query_as::<Postgres, Task>(GET_TASK_QUERY)
                    .bind(&task_id)
                    .fetch_optional(&mut *tx)
                    .await?;

                let task = task.ok_or(Error::NotFound)?;

                let Task {
                    state_machine_states,
                    state_machine_config,
                    ..
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
                                .apply_trigger(task_trigger_id, Some(&payload))
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
                        r##"INSERT INTO action_queue
                        (task_id, task_trigger_id, action_id, payload)
                        VALUES
                        {}
                        "##,
                        sql_insert_parameters::<4>(actions.len())
                    );

                    let mut query = sqlx::query(&q);
                    for action in actions {
                        query = query
                            .bind(action.task_id)
                            .bind(action.task_trigger_id)
                            .bind(action.action_id)
                            .bind(action.payload);
                    }

                    query.execute(&mut *tx).await?;
                }

                Ok::<(), Error>(())
            })
        })
        .await
    }
}

#[derive(Serialize, Deserialize)]
pub struct TaskTrigger {
    pub task_trigger_id: i64,
    pub task_id: i64,
    pub input_id: i64,
    pub last_payload: Option<Box<serde_json::value::RawValue>>,
}
