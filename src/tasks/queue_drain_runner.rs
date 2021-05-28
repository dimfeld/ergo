//! Run all the queue drains

use super::{actions::queue::ActionQueue, inputs::queue::InputQueue};
use crate::{
    database::VaultPostgresPool,
    error::Error,
    graceful_shutdown::GracefulShutdownConsumer,
    queues::postgres_drain::{QueueStageDrain, QueueStageDrainStats},
};

use serde::Serialize;

/// Handle draining task staging tables for both actions and inputs.
pub struct AllQueuesDrain {
    action_drain: QueueStageDrain,
    input_drain: QueueStageDrain,
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
        pg_pool: VaultPostgresPool,
        redis_pool: deadpool_redis::Pool,
        shutdown: GracefulShutdownConsumer,
    ) -> Result<AllQueuesDrain, Error> {
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
        })
    }

    pub fn stats(&self) -> AllQueuesDrainStats {
        let actions = self.action_drain.stats.borrow().clone();
        let inputs = self.input_drain.stats.borrow().clone();
        AllQueuesDrainStats { actions, inputs }
    }
}
