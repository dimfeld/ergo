mod discord_webhook;
mod notification;
pub use notification::*;

use std::{borrow::Cow, sync::Arc};

use smallvec::SmallVec;
use sqlx::PgConnection;

use crate::{
    database::{PostgresPool, RedisPool},
    error::{Error, Result},
    graceful_shutdown::GracefulShutdownConsumer,
    queues::{generic_stage::QueueJob, Queue, QueueJobProcessor},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use self::discord_webhook::send_discord_webhook;

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

pub struct NotificationManager(Arc<NotificationManagerInner>);

impl Clone for NotificationManager {
    fn clone(&self) -> Self {
        NotificationManager(self.0.clone())
    }
}

pub struct NotificationManagerInner {
    pg_pool: PostgresPool,
    shutdown: GracefulShutdownConsumer,
    queue: Queue,
    queue_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct NotificationJob<'a> {
    service: NotifyService,
    destination: String,
    notification: Cow<'a, Notification>,
}

#[derive(sqlx::FromRow)]
struct ServiceAndDestination {
    service: NotifyService,
    destination: String,
}

impl NotificationManager {
    pub fn new(
        pg_pool: PostgresPool,
        redis_pool: RedisPool,
        shutdown: GracefulShutdownConsumer,
    ) -> Result<NotificationManager> {
        let queue_name = match redis_pool.key_prefix() {
            Some(prefix) => format!("{}-{}", prefix, QUEUE_NAME),
            None => QUEUE_NAME.to_string(),
        };

        let queue = Queue::new(redis_pool, queue_name.clone(), None, None, None);
        Ok(NotificationManager(Arc::new(NotificationManagerInner {
            pg_pool,
            shutdown,
            queue,
            queue_name,
        })))
    }

    // Enqueue a notification to be sent
    pub async fn notify(
        &self,
        tx: &mut PgConnection,
        org_id: &uuid::Uuid,
        notification: Notification,
    ) -> Result<()> {
        let notifications = self.get_notifiers(tx, org_id, &notification).await?;

        for sd in notifications {
            let payload = NotificationJob {
                service: sd.service,
                destination: sd.destination,
                notification: Cow::Borrowed(&notification),
            };

            QueueJob::new(self.0.queue_name.as_str(), &payload)
                .enqueue(tx)
                .await?;
        }
        Ok(())
    }

    async fn get_notifiers(
        &self,
        tx: &mut PgConnection,
        org_id: &uuid::Uuid,
        notification: &Notification,
    ) -> Result<Vec<ServiceAndDestination>> {
        let mut object_ids = SmallVec::<[i64; 3]>::new();
        object_ids.push(1);
        object_ids.push(notification.task_id);
        if let Some(object_id) = notification.local_object_id {
            object_ids.push(object_id);
        }

        let notifications = sqlx::query_as!(
            ServiceAndDestination,
            r##"SELECT
          service AS "service: NotifyService", destination
          FROM notify_listeners
          JOIN notify_endpoints USING(notify_endpoint_id, org_id)
          WHERE org_id=$1 AND object_id = ANY($2) AND event=$3"##,
            org_id,
            object_ids.as_slice(),
            notification.event as _,
        )
        .fetch_all(tx)
        .await?;

        Ok(notifications)
    }

    pub fn start_dequeuer_loop(&mut self) -> Result<(), Error> {
        self.0.queue.start_dequeuer_loop(
            self.0.shutdown.clone(),
            None,
            None,
            NotifyExecutor {
                pg_pool: self.0.pg_pool.clone(),
                http_client: reqwest::ClientBuilder::new()
                    .timeout(std::time::Duration::from_secs(30))
                    .build()?,
            },
        );

        Ok(())
    }
}

#[derive(Clone)]
struct NotifyExecutor {
    pg_pool: PostgresPool,
    http_client: reqwest::Client,
}

#[async_trait]
impl QueueJobProcessor for NotifyExecutor {
    type Payload = NotificationJob<'static>;

    async fn process(
        &self,
        item: &crate::queues::QueueWorkItem<Self::Payload>,
    ) -> Result<(), Error> {
        let NotificationJob {
            service,
            destination,
            notification,
        } = &item.data;
        match service {
            NotifyService::Email => Ok(()),
            NotifyService::SlackIncomingWebhook => Ok(()),
            NotifyService::DiscordIncomingWebhook => {
                send_discord_webhook(&self.http_client, destination, notification.as_ref()).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore]
    fn runs_all_notifiers() {}
}
