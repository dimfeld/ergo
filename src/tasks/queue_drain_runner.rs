//! Run all the queue drains

use super::{actions::queue::ActionQueue, inputs::queue::InputQueue};
use crate::{
    database::{VaultPostgresPool, VaultPostgresPoolOptions},
    error::Error,
    queues::postgres_drain::{QueueStageDrain, QueueStageDrainStats},
    service_config::Config,
    vault::VaultClientTokenData,
};

use serde::Serialize;

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
    pub fn new(config: Config) -> Result<AllQueuesDrain, Error> {
        let pg_pool = VaultPostgresPool::new(VaultPostgresPoolOptions {
            max_connections: 16,
            host: config.database_host,
            database: config.database.unwrap_or_else(|| "ergo".to_string()),
            auth: config.database_auth,
            shutdown: config.shutdown.clone(),
        })?;

        let redis_pool = deadpool_redis::Config {
            url: Some(config.redis_host),
            pool: None,
        }
        .create_pool()?;

        let (_, action_drain) = super::actions::queue::new_with_drain(
            pg_pool.clone(),
            redis_pool.clone(),
            config.shutdown.clone(),
        )?;

        let (_, input_drain) = super::inputs::queue::new_with_drain(
            pg_pool.clone(),
            redis_pool.clone(),
            config.shutdown.clone(),
        )?;

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
