//! Read events from the queues and execute tasks

use std::sync::{atomic::AtomicU64, Arc};

use async_trait::async_trait;
use tokio::task::JoinHandle;

use crate::{
    database::PostgresPool, error::Error, graceful_shutdown::GracefulShutdownConsumer,
    queues::QueueJobProcessor,
};

use super::{
    inputs::{queue::InputQueue, InputInvocation},
    Task,
};

pub struct TaskExecutor(Arc<TaskExecutorInner>);

pub struct TaskExecutorInner {
    pg_pool: PostgresPool,
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
        let queue = super::inputs::queue::new(config.redis_pool);

        let executor = TaskExecutor(Arc::new(TaskExecutorInner {
            pg_pool: config.pg_pool,
            queue,
        }));

        executor
            .0
            .queue
            .start_dequeuer_loop(config.shutdown, None, None, executor.clone());

        Ok(executor)
    }
}

impl Clone for TaskExecutor {
    fn clone(&self) -> Self {
        TaskExecutor(self.0.clone())
    }
}

#[async_trait]
impl QueueJobProcessor for TaskExecutor {
    type Payload = InputInvocation;
    async fn process(&self, id: &str, invocation: &InputInvocation) -> Result<(), Error> {
        Task::apply_input(
            &self.0.pg_pool,
            invocation.task_id,
            invocation.input_id,
            invocation.task_trigger_id,
            invocation.payload.clone(),
        )
        .await?;
        Ok(())
    }
}
