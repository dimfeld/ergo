//! Read events from the queues and execute tasks

use std::sync::{atomic::AtomicU64, Arc};

use async_trait::async_trait;
use tokio::task::JoinHandle;

use crate::{
    database::PostgresPool,
    error::Error,
    graceful_shutdown::GracefulShutdownConsumer,
    queues::{QueueJobProcessor, QueueWorkItem},
};

use super::{super::Task, queue::InputQueue, InputInvocation};

pub struct TaskExecutor {
    queue: InputQueue,
}

pub struct TaskExecutorConfig {
    pub pg_pool: PostgresPool,
    pub redis_pool: deadpool_redis::Pool,
    pub shutdown: GracefulShutdownConsumer,
    /// The highest number of concurrent jobs to run. Defaults to twice the number of CPUs.
    pub max_concurrent_jobs: Option<usize>,
}

impl TaskExecutor {
    pub fn new(config: TaskExecutorConfig) -> Result<TaskExecutor, Error> {
        // Start the event queue reader.
        let queue = InputQueue::new(config.redis_pool);

        let executor = TaskExecutor { queue };

        let processor = TaskExecutorJobProcessor {
            pg_pool: config.pg_pool,
        };

        executor
            .queue
            .start_dequeuer_loop(config.shutdown, None, None, processor);

        Ok(executor)
    }
}

#[derive(Clone)]
pub struct TaskExecutorJobProcessor {
    pg_pool: PostgresPool,
}

#[async_trait]
impl QueueJobProcessor for TaskExecutorJobProcessor {
    type Payload = InputInvocation;
    async fn process(&self, item: &QueueWorkItem<InputInvocation>) -> Result<(), Error> {
        let invocation = &item.data;
        Task::apply_input(
            &self.pg_pool,
            invocation.task_id,
            invocation.input_id,
            invocation.task_trigger_id,
            invocation.inputs_log_id,
            invocation.payload.clone(),
        )
        .await?;
        Ok(())
    }
}
