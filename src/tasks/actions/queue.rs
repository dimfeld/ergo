use std::ops::Deref;

use crate::{
    database::{PostgresPool, VaultPostgresPool},
    error::Error,
    graceful_shutdown::GracefulShutdownConsumer,
    queues::{
        postgres_drain::{Drainer, QueueStageDrain, QueueStageDrainConfig},
        Job, JobId, Queue,
    },
};

use async_trait::async_trait;
use sqlx::{query_as, Postgres, Transaction};

struct QueueDrainer {}

#[async_trait]
impl Drainer for QueueDrainer {
    async fn get(&self, tx: &mut Transaction<Postgres>) -> Result<Vec<Job>, Error> {
        let results = sqlx::query!(
            r##"SELECT action_queue_id, task_id, task_trigger_id, action_id, payload
            FROM action_queue ORDER BY action_queue_id LIMIT 50"##
        )
        .fetch_all(&mut *tx)
        .await?;

        if let Some(max_id) = results.last().map(|r| r.action_queue_id) {
            sqlx::query!(
                "DELETE FROM action_queue WHERE action_queue_id < $1",
                max_id
            )
            .execute(&mut *tx)
            .await?;
        }

        results
            .into_iter()
            .map(|row| {
                let payload = super::ActionInvocation {
                    task_id: row.task_id,
                    task_trigger_id: row.task_trigger_id,
                    action_id: row.action_id,
                    payload: row.payload.unwrap_or(serde_json::Value::Null),
                };

                Job::from_json_payload(JobId::Value(&row.action_queue_id.to_string()), &payload)
            })
            .collect::<Result<Vec<Job>, serde_json::Error>>()
            .map_err(Error::from)
    }
}

const QUEUE_NAME: &str = "er-action";

pub struct ActionQueue(Queue);
impl Deref for ActionQueue {
    type Target = Queue;

    fn deref(&self) -> &Queue {
        &self.0
    }
}

pub fn new(redis_pool: deadpool_redis::Pool) -> ActionQueue {
    ActionQueue(Queue::new(redis_pool, QUEUE_NAME, None, None, None))
}

/// Create an action queue and a task to drain the Postgres staging table into the queue.
pub fn new_with_drain(
    db_pool: PostgresPool,
    redis_pool: deadpool_redis::Pool,
    shutdown: GracefulShutdownConsumer,
) -> Result<(ActionQueue, QueueStageDrain), Error> {
    let queue = new(redis_pool);
    let config = QueueStageDrainConfig {
        db_pool,
        drainer: QueueDrainer {},
        queue: queue.clone(),
        shutdown,
    };

    let drain = QueueStageDrain::new(config)?;
    Ok((queue, drain))
}