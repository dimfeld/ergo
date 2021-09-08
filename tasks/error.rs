use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Queue Error {0}")]
    QueueError(#[from] ergo_queues::Error),

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("SQL Error: {0}")]
    SqlError(#[from] sqlx::error::Error),

    #[error(transparent)]
    JsonSchemaCompilationError(#[from] jsonschema::CompilationError),

    #[error("{0:?}")]
    JsonSchemaValidationError(Vec<String>),

    #[error(transparent)]
    DatabaseError(#[from] ergo_database::Error),

    #[error("State Machine Error: {0}")]
    StateMachineError(#[from] crate::state_machine::StateMachineError),

    #[error(transparent)]
    NotificationError(#[from] ergo_notifications::Error),

    #[error(transparent)]
    ExecuteError(#[from] crate::actions::execute::ExecuteError),

    #[error("Not found")]
    NotFound,
}

impl<'a> From<jsonschema::ErrorIterator<'a>> for Error {
    fn from(e: jsonschema::ErrorIterator<'a>) -> Error {
        let inner = e.map(|e| e.to_string()).collect::<Vec<_>>();
        Error::JsonSchemaValidationError(inner)
    }
}

impl ergo_database::transaction::TryIntoSqlxError for Error {
    fn try_into_sqlx_error(self) -> Result<sqlx::Error, Self> {
        match self {
            Self::SqlError(e) => Ok(e),
            _ => Err(self),
        }
    }
}
