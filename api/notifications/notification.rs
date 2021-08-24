use std::borrow::Cow;

use super::Level;

use ergo_database::object_id::TaskId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Notification {
    pub event: NotifyEvent,
    pub task_id: TaskId,
    pub task_name: String,
    pub local_id: String,
    pub local_object_name: String,
    pub local_object_id: Option<Uuid>,
    pub payload: Option<serde_json::Value>,
    pub error: Option<String>,
    pub log_id: Option<Uuid>,
}

impl Notification {
    pub fn fields<'a>(&'a self) -> Vec<(&'static str, Cow<'a, str>, bool)> {
        let mut output = Vec::with_capacity(8);

        output.push(("Task", Cow::from(&self.task_name), true));
        output.push((
            self.event.local_object_type(),
            Cow::from(&self.local_object_name),
            true,
        ));

        if let Some(e) = self.error.as_ref() {
            output.push(("Error", Cow::from(e.as_str()), false));
        }

        if let Some(p) = self.payload.as_ref() {
            let payload = serde_json::to_string(p).unwrap_or(String::new());
            output.push(("Payload", Cow::from(payload), false));
        }

        if let Some(id) = self.log_id.as_ref() {
            output.push(("Log ID", Cow::from(id.to_string()), true));
        }

        output
    }
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "notify_service", rename_all = "snake_case")]
pub enum NotifyService {
    Email,
    DiscordIncomingWebhook,
    SlackIncomingWebhook,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "notify_event", rename_all = "snake_case")]
pub enum NotifyEvent {
    InputArrived,
    InputProcessed,
    ActionStarted,
    ActionSuccess,
    ActionError,
}

impl NotifyEvent {
    pub fn level(&self) -> Level {
        match self {
            Self::InputArrived { .. } => Level::Debug,
            Self::InputProcessed { .. } => Level::Info,
            Self::ActionStarted { .. } => Level::Debug,
            Self::ActionSuccess { .. } => Level::Info,
            Self::ActionError { .. } => Level::Error,
        }
    }

    pub fn local_object_type(&self) -> &'static str {
        match self {
            Self::InputArrived | Self::InputProcessed => "Input",
            Self::ActionStarted | Self::ActionSuccess | Self::ActionError => "Action",
        }
    }
}
