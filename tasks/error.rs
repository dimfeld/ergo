use std::{
    borrow::Cow,
    fmt::{Display, Write},
};

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

pub struct ValidatePath(SmallVec<[ValidatePathSegment; 4]>);

impl ValidatePath {
    pub fn as_inner(&self) -> &SmallVec<[ValidatePathSegment; 4]> {
        &self.0
    }
}

impl ToString for ValidatePath {
    fn to_string(&self) -> String {
        let needed_size = self
            .0
            .iter()
            .map(|p| match p {
                // Enough for brackets plus the digits.
                ValidatePathSegment::Index(i) => {
                    if *i < 10 {
                        3
                    } else if *i < 100 {
                        4
                    } else {
                        5
                    }
                }
                // path segment plus dot
                ValidatePathSegment::String(s) => s.len() + 1,
            })
            .sum();

        let mut output = String::with_capacity(needed_size);
        let mut first = true;
        for p in &self.0 {
            match p {
                ValidatePathSegment::String(s) => {
                    if !first {
                        output.write_char('.').ok();
                    }

                    output.write_str(s.as_ref()).ok();
                }
                ValidatePathSegment::Index(i) => {
                    write!(output, "[{}]", i).ok();
                }
            }
            first = false;
        }

        output
    }
}

pub enum ValidatePathSegment {
    String(Cow<'static, str>),
    Index(usize),
}

#[derive(Clone, Debug, Error)]
pub enum ValidateError {
    #[error("Invalid initial state: {}", .0)]
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

    #[error(
        "Event handler {source}.on[{index}] has invalid target {target}",
        source=.state.as_ref().map(|s| s.as_str()).unwrap_or("<root>")
    )]
    InvalidTarget {
        state: Option<String>,
        index: usize,
        target: String,
    },
}

fn path_segment_for_state(state: &Option<String>) -> SmallVec<[ValidatePathSegment; 4]> {
    let state_name = ValidatePathSegment::String(
        state
            .as_ref()
            .map(|s| Cow::Owned(s.clone()))
            .unwrap_or(Cow::Borrowed("<root>")),
    );

    match state.is_some() {
        true => smallvec![
            ValidatePathSegment::String(Cow::Borrowed("states")),
            state_name
        ],
        false => smallvec![state_name],
    }
}

impl ValidateError {
    pub fn path(&self) -> Option<ValidatePath> {
        match self {
            Self::InvalidInitialState(_) => {
                Some(ValidatePath(smallvec![ValidatePathSegment::String(
                    Cow::Borrowed("initial")
                )]))
            }
            Self::InvalidTriggerId { index, state, .. } => {
                let mut path = path_segment_for_state(state);
                path.extend([
                    ValidatePathSegment::String(Cow::Borrowed("on")),
                    ValidatePathSegment::Index(*index),
                    ValidatePathSegment::String(Cow::Borrowed("trigger_id")),
                ]);
                Some(ValidatePath(path))
            }
            Self::InvalidTarget { state, index, .. } => {
                let mut path = path_segment_for_state(state);
                path.extend([
                    ValidatePathSegment::String(Cow::from("on")),
                    ValidatePathSegment::Index(*index),
                    ValidatePathSegment::String(Cow::from("target")),
                ]);
                Some(ValidatePath(path))
            }
        }
    }

    pub fn expected(&self) -> Option<Cow<'static, str>> {
        match self {
            Self::InvalidInitialState(_) => Some(Cow::from("a state in the `states` object")),
            Self::InvalidTriggerId { .. } => Some(Cow::from("valid trigger id for this task")),
            Self::InvalidTarget { .. } => Some(Cow::from("a state in the `states` object")),
        }
    }
}
