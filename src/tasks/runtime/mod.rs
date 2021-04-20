//! Read events from the queues and execute tasks

use std::sync::atomic::AtomicU64;

use tokio::task::JoinHandle;

use crate::{database::PostgresPool, error::Error, graceful_shutdown::GracefulShutdownConsumer};

use super::inputs::queue::InputQueue;

pub struct TaskExecutor {
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

        queue.start_dequeuer_loop(
            config.shutdown,
            None,
            None,
            |id, data: &Box<serde_json::value::RawValue>| async move { Ok::<(), Error>(()) },
        );

        Ok(TaskExecutor {
            pg_pool: config.pg_pool,
            queue,
        })
    }
}
