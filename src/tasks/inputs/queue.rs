use std::ops::Deref;

use crate::{
    database::PostgresPool,
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
            r##"SELECT event_queue_id, task_id, task_trigger_id, input_id, inputs_log_id, payload
            FROM event_queue ORDER BY event_queue_id LIMIT 50"##
        )
        .fetch_all(&mut *tx)
        .await?;

        if let Some(max_id) = results.last().map(|r| r.event_queue_id) {
            sqlx::query!("DELETE FROM event_queue WHERE event_queue_id <= $1", max_id)
                .execute(&mut *tx)
                .await?;
        }

        results
            .into_iter()
            .map(|row| {
                let payload = super::InputInvocation {
                    task_id: row.task_id,
                    task_trigger_id: row.task_trigger_id,
                    input_id: row.input_id,
                    inputs_log_id: row.inputs_log_id,
                    payload: row.payload.unwrap_or(serde_json::Value::Null),
                };

                Job::from_json_payload(JobId::Value(&row.event_queue_id.to_string()), &payload)
            })
            .collect::<Result<Vec<Job>, serde_json::Error>>()
            .map_err(Error::from)
    }
}

const QUEUE_NAME: &str = "er-input";

#[derive(Clone)]
pub struct InputQueue(Queue);
impl Deref for InputQueue {
    type Target = Queue;

    fn deref(&self) -> &Queue {
        &self.0
    }
}

impl InputQueue {
    pub fn new(redis_pool: deadpool_redis::Pool) -> InputQueue {
        InputQueue(Queue::new(redis_pool, QUEUE_NAME, None, None, None))
    }
}

/// Create an action queue and a task to drain the Postgres staging table into the queue.
pub fn new_drain(
    input_queue: InputQueue,
    db_pool: PostgresPool,
    shutdown: GracefulShutdownConsumer,
) -> Result<QueueStageDrain, Error> {
    let config = QueueStageDrainConfig {
        db_pool,
        drainer: QueueDrainer {},
        queue: input_queue.0,
        shutdown,
    };

    QueueStageDrain::new(config)
}
