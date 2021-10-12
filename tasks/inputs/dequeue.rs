//! Read events from the queues and execute tasks

use std::num::NonZeroU32;

use async_trait::async_trait;
use ergo_database::{PostgresPool, RedisPool};
use ergo_graceful_shutdown::GracefulShutdownConsumer;
use ergo_notifications::NotificationManager;
use ergo_queues::{QueueJobProcessor, QueueWorkItem};

use crate::error::Error;

use super::{super::Task, queue::InputQueue, InputInvocation};

pub struct TaskExecutor {
    queue: InputQueue,
}

pub struct TaskExecutorConfig {
    pub pg_pool: PostgresPool,
    pub redis_pool: RedisPool,
    pub shutdown: GracefulShutdownConsumer,
    pub notifications: Option<NotificationManager>,
    /// The highest number of concurrent jobs to run. Defaults to twice the number of CPUs.
    pub max_concurrent_jobs: Option<usize>,
}

impl TaskExecutor {
    pub fn new(config: TaskExecutorConfig) -> Result<TaskExecutor, Error> {
        let redis_key_prefix = config.redis_pool.key_prefix().map(|s| s.to_string());

        // Start the event queue reader.
        let queue = InputQueue::new(config.redis_pool);

        let executor = TaskExecutor { queue };

        let processor = TaskExecutorJobProcessor {
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
struct TaskExecutorJobProcessor {
    pg_pool: PostgresPool,
    notifications: Option<NotificationManager>,
    redis_key_prefix: Option<String>,
}

#[async_trait]
impl QueueJobProcessor for TaskExecutorJobProcessor {
    type Payload = InputInvocation;
    type Error = Error;

    async fn process(
        &self,
        item: &QueueWorkItem<InputInvocation>,
        invocation: InputInvocation,
    ) -> Result<(), Error> {
        Task::apply_input(
            &self.pg_pool,
            self.notifications.clone(),
            self.redis_key_prefix.clone(),
            item.is_final_retry(),
            invocation,
        )
        .await?;
        Ok(())
    }
}
