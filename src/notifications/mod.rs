mod notification;

use std::borrow::Cow;

use futures::stream::{FuturesUnordered, Stream, StreamExt};
pub use notification::Notification;
use sqlx::{Postgres, Transaction};
use tokio::task::JoinHandle;

use crate::{
    database::PostgresPool,
    error::{Error, Result},
    graceful_shutdown::GracefulShutdownConsumer,
    queues::{generic_stage::QueueJob, Queue, QueueJobProcessor},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

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

const QUEUE_NAME: &'static str = "notifications";

pub struct NotificationManager {
    pg_pool: PostgresPool,
    shutdown: GracefulShutdownConsumer,
    queue: Queue,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "notify_service", rename_all = "snake_case")]
enum NotifyService {
    Email,
    DiscordIncomingWebhook,
    SlackIncomingWebhook,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "notify_event", rename_all = "snake_case")]
enum NotifyEvent {
    InputArrived,
    InputProcessed,
    ActionStarted,
    ActionSuccess,
    ActionError,
}

#[derive(Debug, Serialize, Deserialize)]
struct NotificationJob<'a> {
    service: NotifyService,
    destination: String,
    notification: Cow<'a, Notification>,
}

impl NotificationManager {
    pub fn new(
        pg_pool: PostgresPool,
        redis_pool: deadpool_redis::Pool,
        shutdown: GracefulShutdownConsumer,
    ) -> Result<NotificationManager> {
        let queue = Queue::new(redis_pool, QUEUE_NAME, None, None, None);
        Ok(NotificationManager {
            pg_pool,
            shutdown,
            queue,
        })
    }

    // Enqueue a notification to be sent
    pub async fn notify(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        org_id: &uuid::Uuid,
        notification: Notification,
    ) -> Result<()> {
        #[derive(sqlx::FromRow)]
        struct ServiceAndDestination {
            service: NotifyService,
            destination: String,
        }

        let notifications = sqlx::query_as!(
            ServiceAndDestination,
            r##"SELECT
          service AS "service: NotifyService", destination
          FROM notify_listeners
          JOIN notify_endpoints USING(notify_endpoint_id)
          WHERE org_id=$1 AND object_id IN(1, $2) AND event=$3"##,
            org_id,
            notification.object_id(),
            notification.notify_event() as _,
        )
        .fetch_all(&mut *tx)
        .await?;

        for sd in notifications {
            let payload = NotificationJob {
                service: sd.service,
                destination: sd.destination,
                notification: Cow::Borrowed(&notification),
            };

            QueueJob::new(QUEUE_NAME, &payload).enqueue(tx).await?;
        }
        Ok(())
    }

    pub fn start_dequeuer_loop(&mut self) {
        self.queue.start_dequeuer_loop(
            self.shutdown.clone(),
            None,
            None,
            NotifyExecutor {
                pg_pool: self.pg_pool.clone(),
            },
        );
    }
}

#[derive(Clone)]
struct NotifyExecutor {
    pg_pool: PostgresPool,
}

#[async_trait]
impl QueueJobProcessor for NotifyExecutor {
    type Payload = NotificationJob<'static>;

    async fn process(
        &self,
        item: &crate::queues::QueueWorkItem<Self::Payload>,
    ) -> Result<(), Error> {
        let notification = &item.data;
        Ok(())
    }
}
