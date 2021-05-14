use super::{Level, NotifyEvent};
use crate::error::Error;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
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
        error: Option<String>,
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
        error: String,
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

    pub(super) fn notify_event(&self) -> NotifyEvent {
        match self {
            Self::InputArrived { .. } => NotifyEvent::InputArrived,
            Self::InputProcessed { .. } => NotifyEvent::InputProcessed,
            Self::ActionStarted { .. } => NotifyEvent::ActionStarted,
            Self::ActionSuccess { .. } => NotifyEvent::ActionSuccess,
            Self::ActionError { .. } => NotifyEvent::ActionError,
        }
    }

    pub fn object_id(&self) -> i64 {
        match self {
            Self::InputArrived { task_id, .. } => *task_id,
            Self::InputProcessed { task_id, .. } => *task_id,
            Self::ActionStarted { task_id, .. } => *task_id,
            Self::ActionSuccess { task_id, .. } => *task_id,
            Self::ActionError { task_id, .. } => *task_id,
        }
    }
}
