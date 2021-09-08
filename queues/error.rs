use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Redis error {0}")]
    RedisError(#[from] redis::RedisError),

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("SQL Error")]
    SqlError(#[from] sqlx::error::Error),

    #[error(transparent)]
    DatabaseError(#[from] ergo_database::Error),

    #[error("Redis connection error {0}")]
    RedisPoolError(#[from] deadpool::managed::PoolError<::redis::RedisError>),

    #[error(transparent)]
    ParseBool(#[from] std::str::ParseBoolError),

    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error("Job Error: {0}")]
    JobError(anyhow::Error),

    #[error("Job drain error: {0}")]
    DrainError(anyhow::Error),
}
