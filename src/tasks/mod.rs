pub mod actions;
pub mod handlers;
pub mod inputs;
mod state_machine;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Task {
    pub task_id: i64,
    pub external_task_id: String,
    pub org_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub state_machine_config: state_machine::StateMachineConfig,
    pub state_machine_states: state_machine::StateMachineStates,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct TaskTrigger {
    pub task_trigger_id: i64,
    pub task_id: i64,
    pub input_id: i64,
    pub last_payload: Option<Box<serde_json::value::RawValue>>,
}
