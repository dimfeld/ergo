pub mod actions;
pub mod events;
mod state_machine;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Task {
    task_id: i64,
    external_task_id: String,
    org_id: i64,
    name: String,
    description: Option<String>,
    enabled: bool,
    state_machine_config: state_machine::StateMachineConfig,
    state_machine_states: state_machine::StateMachineStates,
    created: DateTime<Utc>,
    modified: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct TaskTrigger {
    task_trigger_id: i64,
    task_id: i64,
    input_id: i64,
    last_payload: Option<serde_json::Value>,
}
