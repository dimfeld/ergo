pub mod actions;
pub mod handlers;
pub mod inputs;
mod state_machine;

pub use state_machine::StateMachineError;

use crate::{database::VaultPostgresPool, error::Error};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{query_as, types::Json, FromRow, Postgres};
use uuid::Uuid;

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
    pub async fn from_db(
        pool: &VaultPostgresPool<()>,
        task_id: i64,
    ) -> Result<Option<Task>, Error> {
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
        pool: &VaultPostgresPool<()>,
        task_id: i64,
        input_id: i64,
        task_trigger_id: i64,
        payload: &serde_json::Value,
    ) -> Result<(), Error> {
        unimplemented!();
    }
}

#[derive(Serialize, Deserialize)]
pub struct TaskTrigger {
    pub task_trigger_id: i64,
    pub task_id: i64,
    pub input_id: i64,
    pub last_payload: Option<Box<serde_json::value::RawValue>>,
}
