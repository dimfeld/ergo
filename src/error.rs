use actix_web::{dev::Body, http::StatusCode, HttpResponse};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Authentication failure")]
    AuthenticationError,
    #[error("Unauthorized")]
    AuthorizationError,

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

    #[error("Unspecified")]
    Unspecified,
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
        HttpResponse::InternalServerError().body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Error::AuthenticationError => StatusCode::UNAUTHORIZED,
            Error::AuthorizationError => StatusCode::FORBIDDEN,
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
