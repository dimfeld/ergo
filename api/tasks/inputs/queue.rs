use std::{borrow::Cow, ops::Deref};

use crate::{
    database::{PostgresPool, RedisPool},
    error::Error,
    queues::{
        postgres_drain::{Drainer, QueueStageDrain, QueueStageDrainConfig},
        Job, JobId, Queue,
    },
};

use async_trait::async_trait;
use ergo_graceful_shutdown::GracefulShutdownConsumer;
use sqlx::{Postgres, Transaction};

struct QueueDrainer {}

#[async_trait]
impl Drainer for QueueDrainer {
    fn lock_key(&self) -> i64 {
        79034890
    }

    async fn get(
        &self,
        tx: &mut Transaction<Postgres>,
    ) -> Result<Vec<(Cow<'static, str>, Job)>, Error> {
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

                let job = Job::from_json_payload(
                    JobId::Value(&row.event_queue_id.to_string()),
                    &payload,
                )?;

                Ok::<_, Error>((Cow::Borrowed(QUEUE_NAME), job))
            })
            .collect::<Result<Vec<_>, Error>>()
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
    pub fn new(redis_pool: RedisPool) -> InputQueue {
        let queue_name = match redis_pool.key_prefix() {
            Some(prefix) => format!("{}-{}", prefix, QUEUE_NAME),
            None => QUEUE_NAME.to_string(),
        };

        InputQueue(Queue::new(redis_pool, queue_name, None, None, None))
    }
}

/// Create an action queue and a task to drain the Postgres staging table into the queue.
pub fn new_drain(
    input_queue: InputQueue,
    db_pool: PostgresPool,
    redis_pool: RedisPool,
    shutdown: GracefulShutdownConsumer,
) -> Result<QueueStageDrain, Error> {
    let config = QueueStageDrainConfig {
        db_pool,
        redis_pool,
        drainer: QueueDrainer {},
        queue: Some(input_queue.0),
        shutdown,
    };

    QueueStageDrain::new(config)
}
