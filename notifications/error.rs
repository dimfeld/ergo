use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),

    #[error("Queue Error: {0}")]
    QueueError(#[from] ergo_queues::Error),

    #[error("SQL Error: {0}")]
    SqlError(#[from] sqlx::error::Error),
}
