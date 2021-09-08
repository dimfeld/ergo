//! Run all the queue drains

use crate::{actions::queue::ActionQueue, error::Error, inputs::queue::InputQueue};

use ergo_database::{RedisPool, RenewablePostgresPool};
use ergo_graceful_shutdown::GracefulShutdownConsumer;
use ergo_queues::{
    generic_stage,
    postgres_drain::{QueueStageDrain, QueueStageDrainConfig, QueueStageDrainStats},
};
use serde::Serialize;

/// Handle draining task staging tables for both actions and inputs.
pub struct AllQueuesDrain {
    action_drain: QueueStageDrain,
    input_drain: QueueStageDrain,
    generic_drain: QueueStageDrain,
}

#[derive(Debug, Serialize)]
pub struct AllQueuesDrainStats {
    pub actions: QueueStageDrainStats,
    pub inputs: QueueStageDrainStats,
}

impl AllQueuesDrain {
    pub fn new(
        input_queue: InputQueue,
        action_queue: ActionQueue,
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

        let action_drain = super::actions::queue::new_drain(
            action_queue,
            pg_pool.clone(),
            redis_pool.clone(),
            shutdown.clone(),
        )?;

        let input_drain =
            super::inputs::queue::new_drain(input_queue, pg_pool, redis_pool, shutdown.clone())?;

        Ok(AllQueuesDrain {
            action_drain,
            input_drain,
            generic_drain,
        })
    }

    pub fn stats(&self) -> AllQueuesDrainStats {
        let actions = self.action_drain.stats.borrow().clone();
        let inputs = self.input_drain.stats.borrow().clone();
        AllQueuesDrainStats { actions, inputs }
    }
}
