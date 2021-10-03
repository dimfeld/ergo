use actix_web::{http::StatusCode, HttpResponse};
use envoption::EnvOptionError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Authentication failure")]
    AuthenticationError,

    #[error("Unauthorized")]
    AuthorizationError,

    #[error("Password hasher error: {0}")]
    PasswordHasherError(String),

    #[error("Environment variable error: {0}")]
    EnvOptionError(String),

    #[error("SQL Error")]
    SqlError(#[from] sqlx::error::Error),

    #[error(transparent)]
    DatabaseError(#[from] ergo_database::Error),
}

impl<T: std::error::Error> From<EnvOptionError<T>> for Error {
    fn from(e: EnvOptionError<T>) -> Self {
        Self::EnvOptionError(e.to_string())
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
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
