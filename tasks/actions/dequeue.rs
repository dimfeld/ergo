use async_trait::async_trait;
use ergo_database::{PostgresPool, RedisPool};
use ergo_graceful_shutdown::GracefulShutdownConsumer;
use ergo_notifications::NotificationManager;
use ergo_queues::{QueueJobProcessor, QueueWorkItem};
use std::num::NonZeroU32;

use crate::error::Error;

use super::{execute::execute, queue::ActionQueue, ActionInvocation};

pub struct ActionExecutorConfig {
    pub pg_pool: PostgresPool,
    pub redis_pool: RedisPool,
    pub shutdown: GracefulShutdownConsumer,
    pub notifications: Option<NotificationManager>,
    /// The highest number of concurrent jobs to run. Defaults to twice the number of CPUs.
    pub max_concurrent_jobs: Option<usize>,
}

pub struct ActionExecutor {
    queue: ActionQueue,
}

impl ActionExecutor {
    pub fn new(config: ActionExecutorConfig) -> Result<ActionExecutor, Error> {
        let redis_key_prefix = config.redis_pool.key_prefix().map(|e| e.to_string());
        let queue = ActionQueue::new(config.redis_pool);
        let executor = ActionExecutor { queue };
        let processor = ActionExecutorJobProcessor {
            pg_pool: config.pg_pool,
            notifications: config.notifications,
            redis_key_prefix,
        };

        executor.queue.start_dequeuer_loop(
            config.shutdown,
            None,
            config
                .max_concurrent_jobs
                .and_then(|n| NonZeroU32::new(n as u32)),
            processor,
        );

        Ok(executor)
    }
}

#[derive(Clone)]
struct ActionExecutorJobProcessor {
    pg_pool: PostgresPool,
    notifications: Option<NotificationManager>,
    redis_key_prefix: Option<String>,
}

#[async_trait]
impl QueueJobProcessor for ActionExecutorJobProcessor {
    type Payload = ActionInvocation;
    type Error = Error;

    async fn process(
        &self,
        item: &QueueWorkItem<Self::Payload>,
        data: ActionInvocation,
    ) -> Result<(), Error> {
        execute(
            &self.pg_pool,
            self.redis_key_prefix.clone(),
            self.notifications.as_ref(),
            data,
        )
        .await?;
        Ok(())
    }
}
