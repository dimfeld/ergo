use super::Level;
use crate::error::Error;

#[derive(Debug)]
pub enum Notification {
    InputArrived {
        task_id: i64,
        task_trigger_local_id: String,
        payload: serde_json::Value,
    },

    InputProcessed {
        task_id: i64,
        task_trigger_local_id: String,
        payload: serde_json::Value,
        error: Option<Error>,
    },

    ActionStarted {
        task_id: i64,
        task_action_local_id: String,
        payload: serde_json::Value,
    },

    ActionSuccess {
        task_id: i64,
        task_action_local_id: String,
        payload: serde_json::Value,
    },

    ActionError {
        task_id: i64,
        task_action_local_id: String,
        payload: serde_json::Value,
        error: Error,
    },
}

impl Notification {
    pub fn level(&self) -> Level {
        match self {
            Self::InputArrived { .. } => Level::Debug,
            Self::InputProcessed { .. } => Level::Info,
            Self::ActionStarted { .. } => Level::Debug,
            Self::ActionSuccess { .. } => Level::Info,
            Self::ActionError { .. } => Level::Error,
        }
    }
}
