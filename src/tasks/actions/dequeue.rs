use async_trait::async_trait;
use std::num::NonZeroU32;

use crate::{
    database::PostgresPool,
    error::Error,
    graceful_shutdown::GracefulShutdownConsumer,
    queues::{QueueJobProcessor, QueueWorkItem},
};

use super::{execute::execute, queue::ActionQueue, ActionInvocation};

pub struct ActionExecutorConfig {
    pub pg_pool: PostgresPool,
    pub redis_pool: deadpool_redis::Pool,
    pub shutdown: GracefulShutdownConsumer,
    /// The highest number of concurrent jobs to run. Defaults to twice the number of CPUs.
    pub max_concurrent_jobs: Option<usize>,
}

pub struct ActionExecutor {
    queue: ActionQueue,
}

impl ActionExecutor {
    pub fn new(config: ActionExecutorConfig) -> Result<ActionExecutor, Error> {
        let queue = ActionQueue::new(config.redis_pool);
        let executor = ActionExecutor { queue };
        let processor = ActionExecutorJobProcessor {
            pg_pool: config.pg_pool,
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
}

#[async_trait]
impl QueueJobProcessor for ActionExecutorJobProcessor {
    type Payload = ActionInvocation;

    async fn process(&self, item: &QueueWorkItem<Self::Payload>) -> Result<(), Error> {
        let invocation = &item.data;
        execute(&self.pg_pool, &item.data).await?;
        Ok(())
    }
}