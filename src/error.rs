use actix_web::{http::StatusCode, HttpResponse};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
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
}

impl actix_web::error::ResponseError for Error {
    fn error_response(&self) -> HttpResponse<actix_web::dev::Body> {
        HttpResponse::InternalServerError().body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
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
