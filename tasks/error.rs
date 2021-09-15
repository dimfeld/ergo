use std::{borrow::Cow, fmt::Display};

use smallvec::{smallvec, SmallVec};
use thiserror::Error;
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[derive(Debug, Error)]
pub enum Error {
    #[cfg(feature = "full")]
    #[error("Queue Error {0}")]
    QueueError(#[from] ergo_queues::Error),

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[cfg(feature = "full")]
    #[error("SQL Error: {0}")]
    SqlError(#[from] sqlx::error::Error),

    #[error("{0:?}")]
    JsonSchemaValidationError(SmallVec<[String; 2]>),

    #[cfg(feature = "full")]
    #[error(transparent)]
    DatabaseError(#[from] ergo_database::Error),

    #[error("State Machine Error: {0}")]
    StateMachineError(#[from] crate::state_machine::StateMachineError),

    #[cfg(feature = "full")]
    #[error(transparent)]
    NotificationError(#[from] ergo_notifications::Error),

    #[cfg(feature = "full")]
    #[error(transparent)]
    ExecuteError(#[from] crate::actions::execute::ExecuteError),

    #[error("Not found")]
    NotFound,

    #[error("Task validation errors: {0}")]
    ValidateError(#[from] ValidateErrors),
}

impl<'a> From<jsonschema::ErrorIterator<'a>> for Error {
    fn from(e: jsonschema::ErrorIterator<'a>) -> Error {
        let inner = e.map(|e| e.to_string()).collect::<SmallVec<_>>();
        Error::JsonSchemaValidationError(inner)
    }
}

impl<'a> From<jsonschema::ValidationError<'a>> for Error {
    fn from(e: jsonschema::ValidationError<'a>) -> Error {
        Error::JsonSchemaValidationError(smallvec![e.to_string()])
    }
}

#[cfg(feature = "sqlx")]
impl ergo_database::transaction::TryIntoSqlxError for Error {
    fn try_into_sqlx_error(self) -> Result<sqlx::Error, Self> {
        match self {
            Self::SqlError(e) => Ok(e),
            _ => Err(self),
        }
    }
}

#[derive(Debug)]
pub struct ValidateErrors(pub Vec<ValidateError>);
impl std::error::Error for ValidateErrors {}
impl std::fmt::Display for ValidateErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for err in self.0.iter() {
            writeln!(f, "{}", err)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Error)]
pub enum ValidateError {
    #[error("Invalid initial state ({})", 0)]
    InvalidInitialState(String),

    #[error(
        "Event handler {source}.on[{index}] has unknown trigger id {trigger_id}",
        source=.state.as_ref().map(|s| s.as_str()).unwrap_or("<root>")
    )]
    InvalidTriggerId {
        trigger_id: String,
        index: usize,
        state: Option<String>,
    },
}

impl ValidateError {
    pub fn path(&self) -> Option<Vec<Cow<'static, str>>> {
        match self {
            Self::InvalidInitialState(_) => None,
            Self::InvalidTriggerId { index, state, .. } => Some(vec![
                state
                    .as_ref()
                    .map(|s| Cow::Owned(s.clone()))
                    .unwrap_or(Cow::Borrowed("<root>")),
                Cow::Borrowed("on"),
                Cow::from(format!("{}", index)),
            ]),
        }
    }

    pub fn expected(&self) -> Option<Cow<'static, str>> {
        match self {
            Self::InvalidInitialState(_) => None,
            Self::InvalidTriggerId { .. } => Some(Cow::from("valid trigger id for this task")),
        }
    }
}
