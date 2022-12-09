use std::str::ParseBoolError;

use actix_web::{http::StatusCode, HttpResponse};
use envoption::EnvOptionError;
use ergo_tasks::{actions::template::TemplateError, state_machine::StateMachineError};
use smallvec::{smallvec, SmallVec};
use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Configuration Error: {0}")]
    ConfigError(String),

    #[error("Authentication failure")]
    AuthenticationError,
    #[error("Unauthorized")]
    AuthorizationError,

    #[error("Not found")]
    NotFound,

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),

    #[error("Actix error {:?}", body)]
    ActixError {
        status_code: StatusCode,
        body: String,
    },

    #[error("Redis error {0}")]
    RedisError(#[from] redis::RedisError),

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[error(transparent)]
    UuidError(#[from] uuid::Error),

    #[error("{0:?}")]
    JsonSchemaValidationError(SmallVec<[String; 2]>),

    #[error("State Machine Error: {0}")]
    StateMachineError(#[from] StateMachineError),

    #[error(transparent)]
    ParseBool(#[from] ParseBoolError),

    #[error("Unknown executor {0}")]
    UnknownExecutor(String),

    #[error(transparent)]
    TemplateError(#[from] TemplateError),

    #[error("Environment variable error: {0}")]
    EnvOptionError(String),

    #[error("Redis connection error {0}")]
    RedisPoolError(#[from] deadpool::managed::PoolError<::redis::RedisError>),

    #[error("SQL Error: {0}")]
    SqlError(#[from] sqlx::error::Error),

    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),

    #[error("Script Error: {0}")]
    ScriptError(anyhow::Error),

    #[error(transparent)]
    DatabaseError(#[from] ergo_database::Error),

    #[error(transparent)]
    AuthError(#[from] ergo_auth::Error),

    #[error("{0}")]
    StringError(String),

    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error("Queue Error: {0}")]
    QueueError(#[from] ergo_queues::Error),

    #[error(transparent)]
    TasksError(#[from] ergo_tasks::Error),

    #[error(transparent)]
    NotificationError(#[from] ergo_notifications::Error),
}

impl<T: std::error::Error> From<EnvOptionError<T>> for Error {
    fn from(e: EnvOptionError<T>) -> Self {
        Self::EnvOptionError(e.to_string())
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

impl From<actix_web::Error> for Error {
    fn from(e: actix_web::Error) -> Error {
        let r = e.as_response_error();
        Error::ActixError {
            status_code: r.status_code(),
            body: r.to_string(),
        }
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

impl actix_web::error::ResponseError for Error {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Error::AuthenticationError => StatusCode::UNAUTHORIZED,
            Error::AuthorizationError => StatusCode::FORBIDDEN,
            Error::AuthError(ergo_auth::Error::AuthenticationError) => StatusCode::UNAUTHORIZED,
            Error::AuthError(ergo_auth::Error::AuthorizationError) => StatusCode::FORBIDDEN,
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::UnknownExecutor(_) => StatusCode::BAD_REQUEST,
            Error::ActixError { status_code, .. } => *status_code,
            Error::TasksError(ergo_tasks::Error::NotFound) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
