use std::str::ParseBoolError;

use crate::tasks::{
    actions::{execute::ExecuteError, template::TemplateError},
    StateMachineError,
};
use actix_web::{http::StatusCode, HttpResponse};
use envoption::EnvOptionError;
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

    #[error("SQL Error")]
    SqlError(#[from] sqlx::error::Error),

    #[error("Vault Error")]
    VaultError(#[from] hashicorp_vault::Error),

    #[error("Vault returned no auth data")]
    VaultNoDataError,

    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),

    #[error("timed out")]
    TimeoutError,

    #[error("Actix error {:?}", body)]
    ActixError {
        status_code: StatusCode,
        body: String,
    },

    #[error("Redis error {0}")]
    RedisError(#[from] redis::RedisError),

    #[error("Redis connection error {0}")]
    RedisPoolError(#[from] deadpool::managed::PoolError<redis::RedisError>),

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[error(transparent)]
    UuidError(#[from] uuid::Error),

    #[error(transparent)]
    JsonSchemaCompilationError(#[from] jsonschema::CompilationError),

    #[error("{0:?}")]
    JsonSchemaValidationError(Vec<String>),

    #[error("State Machine Error: {0}")]
    StateMachineError(#[from] StateMachineError),

    #[error("Unable to execute serializable transaction")]
    SerializationFailure,

    #[error(transparent)]
    ParseBool(#[from] ParseBoolError),

    #[error("{0}")]
    StringError(String),

    #[error(transparent)]
    ExecuteError(#[from] ExecuteError),

    #[error(transparent)]
    TemplateError(#[from] TemplateError),

    #[error("Environment variable error: {0}")]
    EnvOptionError(String),

    #[error("Password hasher error: {0}")]
    PasswordHasherError(String),
}

impl<T: std::error::Error> From<EnvOptionError<T>> for Error {
    fn from(e: EnvOptionError<T>) -> Self {
        Self::EnvOptionError(e.to_string())
    }
}

impl<'a> From<jsonschema::ErrorIterator<'a>> for Error {
    fn from(e: jsonschema::ErrorIterator<'a>) -> Error {
        let inner = e.map(|e| e.to_string()).collect::<Vec<_>>();
        Error::JsonSchemaValidationError(inner)
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

impl actix_web::error::ResponseError for Error {
    fn error_response(&self) -> HttpResponse<actix_web::dev::Body> {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Error::AuthenticationError => StatusCode::UNAUTHORIZED,
            Error::AuthorizationError => StatusCode::FORBIDDEN,
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::ActixError { status_code, .. } => *status_code,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl sqlx::error::DatabaseError for Error {
    fn message(&self) -> &str {
        match self {
            Error::SqlError(sqlx::Error::Database(e)) => e.message(),
            _ => "",
        }
    }

    fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        self
    }

    fn as_error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
        self
    }

    fn into_error(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync + 'static> {
        self
    }
}
