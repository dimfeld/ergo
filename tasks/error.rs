use std::{borrow::Cow, fmt::Write};

#[cfg(not(target_family = "wasm"))]
use ergo_js::ConsoleMessage;
use smallvec::{smallvec, SmallVec};
use thiserror::Error;

use crate::actions::template::TemplateError;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error {
    #[cfg(not(target_family = "wasm"))]
    #[error("Queue Error {0}")]
    QueueError(#[from] ergo_queues::Error),

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[cfg(not(target_family = "wasm"))]
    #[error("SQL Error: {0}")]
    SqlError(#[from] sqlx::error::Error),

    #[error("{0:?}")]
    JsonSchemaValidationError(SmallVec<[String; 2]>),

    #[cfg(not(target_family = "wasm"))]
    #[error(transparent)]
    DatabaseError(#[from] ergo_database::Error),

    #[error("State Machine Error: {0}")]
    StateMachineError(#[from] crate::state_machine::StateMachineError),

    #[cfg(not(target_family = "wasm"))]
    #[error(transparent)]
    NotificationError(#[from] ergo_notifications::Error),

    #[cfg(not(target_family = "wasm"))]
    #[error(transparent)]
    ExecuteError(#[from] crate::actions::execute::ExecuteError),

    #[error("Not found")]
    NotFound,

    #[error("Task validation errors: {0}")]
    TaskValidateError(#[from] TaskValidateErrors),

    #[error("Action validation errors: {0}")]
    ActionValidateError(#[from] ActionValidateErrors),

    #[error("No task action found with name {0}")]
    TaskActionNotFound(String),

    #[error("No task trigger found with name {0}")]
    TaskTriggerNotFound(String),

    #[error("Config is for task type {0} and state is of a different type")]
    ConfigStateMismatch(&'static str),

    #[error("Setting up task script: {0}")]
    TaskScriptSetup(anyhow::Error),

    #[error("Task script error: {error}")]
    #[cfg(not(target_family = "wasm"))]
    TaskScript {
        #[source]
        error: ergo_js::Error,
        console: Vec<ConsoleMessage>,
    },

    #[error("Failed to initialize dataflow environment: {error}")]
    #[cfg(not(target_family = "wasm"))]
    DataflowInitScriptError {
        #[source]
        error: ergo_js::Error,
    },

    #[error("Dataflow node {node} script error: {error}")]
    #[cfg(not(target_family = "wasm"))]
    DataflowScript {
        node: String,
        #[source]
        error: ergo_js::Error,
        console: Vec<ConsoleMessage>,
    },

    #[error("Failed to read result from node {node}: {error}")]
    #[cfg(not(target_family = "wasm"))]
    DataflowGetStateError {
        node: String,
        #[source]
        error: ergo_js::Error,
    },

    #[error("Failed to write state for node {node}: {error}")]
    #[cfg(not(target_family = "wasm"))]
    DataflowSetStateError {
        node: String,
        #[source]
        error: ergo_js::Error,
    },

    #[error("Parsing cron schedule: {0}")]
    CronParseError(#[from] cron::error::Error),

    #[error("Tried to run empty task")]
    TaskIsEmpty,

    #[error("Node {0} does not exist")]
    MissingDataFlowNode(u32),

    #[error("Node {0} does not exist")]
    MissingDataFlowNodeName(String),

    #[error("Node {0} depends on {1}, which does not exist")]
    BadEdgeIndex(u32, u32),

    #[error("Node {0} has a cyclic dependency")]
    DataflowCycle(u32),

    #[error("Periodic task was deleted")]
    PeriodicTaskDeleted,

    #[cfg(target_family = "wasm")]
    #[error(transparent)]
    JsSerdeError(#[from] serde_wasm_bindgen::Error),

    #[cfg(target_family = "wasm")]
    #[error("JS Error")]
    JsError(wasm_bindgen::JsValue),
}

#[cfg(target_family = "wasm")]
impl From<wasm_bindgen::JsValue> for Error {
    fn from(value: wasm_bindgen::JsValue) -> Self {
        Self::JsError(value)
    }
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

#[cfg(not(target_family = "wasm"))]
impl ergo_database::transaction::TryIntoSqlxError for Error {
    fn try_into_sqlx_error(self) -> Result<sqlx::Error, Self> {
        match self {
            Self::SqlError(e) => Ok(e),
            _ => Err(self),
        }
    }
}

#[derive(Debug)]
pub struct TaskValidateErrors(pub Vec<TaskValidateError>);
impl std::error::Error for TaskValidateErrors {}
impl std::fmt::Display for TaskValidateErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for err in self.0.iter() {
            writeln!(f, "{}", err)?;
        }
        Ok(())
    }
}

pub type ValidatePathSegments = SmallVec<[ValidatePathSegment; 8]>;
pub struct ValidatePath(ValidatePathSegments);

impl ValidatePath {
    pub fn as_inner(&self) -> &ValidatePathSegments {
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

impl From<String> for ValidatePathSegment {
    fn from(s: String) -> Self {
        Self::String(Cow::Owned(s))
    }
}

impl From<&'static str> for ValidatePathSegment {
    fn from(s: &'static str) -> Self {
        Self::String(Cow::Borrowed(s))
    }
}

impl From<usize> for ValidatePathSegment {
    fn from(i: usize) -> Self {
        Self::Index(i)
    }
}

#[derive(Clone, Debug, Error)]
pub enum TaskValidateError {
    #[error("Invalid initial state: {}", .0)]
    InvalidInitialState(String),

    #[error(
        "Event handler {source}.on[{index}] has unknown trigger id {trigger_id}",
        source=.state.as_deref().unwrap_or("<root>")
    )]
    InvalidTriggerId {
        trigger_id: String,
        index: usize,
        state: Option<String>,
    },

    #[error(
        "Event handler {source}.on[{index}] has invalid target {target}",
        source=.state.as_deref().unwrap_or("<root>")
    )]
    InvalidTarget {
        state: Option<String>,
        index: usize,
        target: String,
    },
}

fn path_segment_for_state(state: &Option<String>) -> ValidatePathSegments {
    state
        .as_ref()
        .map(|state_name| smallvec!["states".into(), state_name.clone().into(),])
        .unwrap_or_else(SmallVec::new)
}

impl TaskValidateError {
    pub fn path(&self) -> Option<ValidatePath> {
        match self {
            Self::InvalidInitialState(_) => {
                Some(ValidatePath(smallvec![ValidatePathSegment::String(
                    Cow::Borrowed("initial")
                )]))
            }
            Self::InvalidTriggerId { index, state, .. } => {
                let mut path = path_segment_for_state(state);
                path.extend(["on".into(), (*index).into(), "trigger_id".into()]);
                Some(ValidatePath(path))
            }
            Self::InvalidTarget { state, index, .. } => {
                let mut path = path_segment_for_state(state);
                path.extend(["on".into(), (*index).into(), "target".into()]);
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

#[derive(Debug, Error)]
pub enum ActionValidateError {
    #[error("Unknown executor {0}")]
    UnknownExecutor(String),

    #[cfg(not(target_family = "wasm"))]
    #[error("Script error: {0}")]
    ScriptError(ergo_js::Error),
    #[cfg(target_family = "wasm")]
    #[error(transparent)]
    ScriptError(anyhow::Error),

    #[error("Template error: {0}")]
    TemplateError(#[from] TemplateError),
}

impl ActionValidateError {
    pub fn path(&self) -> Option<ValidatePath> {
        match self {
            Self::UnknownExecutor(_) => Some(ValidatePath(smallvec!["executor_id".into()])),
            Self::ScriptError(_) => Some(ValidatePath(smallvec![
                "executor_template".into(),
                "c".into(),
            ])),
            Self::TemplateError(TemplateError::Validation(_)) => {
                // TODO Take data from the template error
                Some(ValidatePath(smallvec![
                    "executor_template".into(),
                    "c".into(),
                ]))
            }
            Self::TemplateError(_) => None,
        }
    }

    pub fn expected(&self) -> Option<Cow<'static, str>> {
        match self {
            Self::UnknownExecutor(_) => None,
            Self::ScriptError(_) => None,
            // TODO Take info from the template error
            Self::TemplateError(_) => None,
        }
    }
}

#[derive(Debug)]
pub struct ActionValidateErrors(pub SmallVec<[ActionValidateError; 1]>);
impl std::error::Error for ActionValidateErrors {}
impl std::fmt::Display for ActionValidateErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for err in self.0.iter() {
            writeln!(f, "{}", err)?;
        }
        Ok(())
    }
}

impl From<ActionValidateError> for ActionValidateErrors {
    fn from(err: ActionValidateError) -> Self {
        Self(smallvec![err])
    }
}
