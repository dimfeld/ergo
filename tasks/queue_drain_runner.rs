//! Run all the queue drains

use crate::error::Error;

use ergo_database::{RedisPool, RenewablePostgresPool};
use ergo_graceful_shutdown::GracefulShutdownConsumer;
use ergo_queues::{
    generic_stage,
    postgres_drain::{QueueStageDrain, QueueStageDrainConfig},
};

pub struct AllQueuesDrain {
    generic_drain: QueueStageDrain,
}

impl AllQueuesDrain {
    pub fn new(
        pg_pool: RenewablePostgresPool,
        redis_pool: RedisPool,
        shutdown: GracefulShutdownConsumer,
    ) -> Result<AllQueuesDrain, Error> {
        let generic_drain = QueueStageDrain::new(QueueStageDrainConfig {
            queue: None,
            drainer: generic_stage::QueueDrainer {},
            db_pool: pg_pool.clone(),
            redis_pool: redis_pool.clone(),
            shutdown: shutdown.clone(),
        })?;

        Ok(AllQueuesDrain { generic_drain })
    }
}
