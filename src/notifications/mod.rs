mod notification;

use futures::stream::{FuturesUnordered, Stream, StreamExt};
pub use notification::Notification;
use tokio::task::JoinHandle;

use crate::{
    database::PostgresPool,
    error::{Error, Result},
    graceful_shutdown::GracefulShutdownConsumer,
};
use async_trait::async_trait;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Debug,
    Info,
    Warning,
    Error,
}

#[async_trait]
trait Notifier {
    async fn notify(&self, notification: &Notification) -> Result<()>;
}

pub struct NotificationManager {
    pg_pool: PostgresPool,
    shutdown: GracefulShutdownConsumer,
    active_tasks: FuturesUnordered<JoinHandle<()>>,
}

impl NotificationManager {
    pub fn new(
        pg_pool: PostgresPool,
        shutdown: GracefulShutdownConsumer,
    ) -> Result<NotificationManager> {
        Ok(NotificationManager {
            pg_pool,
            shutdown,
            active_tasks: FuturesUnordered::new(),
        })
    }

    // Send a notification to all enabled sources.
    pub async fn notify(&self, notification: Notification) {
        // tokio::spawn(async move {})
    }
}
