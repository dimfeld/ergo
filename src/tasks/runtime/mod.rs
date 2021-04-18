//! Read events from the queues and execute tasks

mod dequeue_loop;

use std::sync::atomic::AtomicU64;

use tokio::task::JoinHandle;

use crate::{database::PostgresPool, error::Error, graceful_shutdown::GracefulShutdownConsumer};

pub struct TaskExecutor {
    pg_pool: PostgresPool,
    redis_pool: deadpool_redis::Pool,
    shutdown: GracefulShutdownConsumer,
    active_jobs: AtomicU64,
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

        Ok(TaskExecutor {
            pg_pool: config.pg_pool,
            redis_pool: config.redis_pool,
            shutdown: config.shutdown,
            active_jobs: AtomicU64::new(0),
        })
    }
}
