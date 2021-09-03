use std::{borrow::Cow, ops::Deref};

use crate::{
    error::Error,
    queues::{
        postgres_drain::{Drainer, QueueStageDrain, QueueStageDrainConfig},
        Job, JobId, Queue,
    },
};

use async_trait::async_trait;
use ergo_database::{object_id::TaskId, PostgresPool, RedisPool};
use ergo_graceful_shutdown::GracefulShutdownConsumer;
use sqlx::{Postgres, Transaction};

struct QueueDrainer {}

#[async_trait]
impl Drainer for QueueDrainer {
    fn lock_key(&self) -> i64 {
        67982349
    }

    async fn get(
        &self,
        tx: &mut Transaction<Postgres>,
    ) -> Result<Vec<(Cow<'static, str>, Job)>, Error> {
        let results = sqlx::query!(
            r##"SELECT action_queue_id, actions_log_id,
                task_id as "task_id: TaskId",
                task_action_local_id, input_arrival_id, timeout, payload
            FROM action_queue ORDER BY action_queue_id LIMIT 50"##
        )
        .fetch_all(&mut *tx)
        .await?;

        if let Some(max_id) = results.last().map(|r| r.action_queue_id) {
            sqlx::query!(
                "DELETE FROM action_queue WHERE action_queue_id <= $1",
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
                    task_action_local_id: row.task_action_local_id,
                    actions_log_id: row.actions_log_id,
                    input_arrival_id: row.input_arrival_id,
                    payload: row.payload.unwrap_or(serde_json::Value::Null),
                };

                let mut job = Job::from_json_payload(
                    JobId::Value(&row.action_queue_id.to_string()),
                    &payload,
                )?;

                job.timeout = row
                    .timeout
                    .map(|t| std::time::Duration::from_secs(t as u64));

                Ok::<_, Error>((Cow::Borrowed(QUEUE_NAME), job))
            })
            .collect::<Result<Vec<_>, Error>>()
    }
}

const QUEUE_NAME: &str = "er-action";

#[derive(Clone)]
pub struct ActionQueue(Queue);
impl Deref for ActionQueue {
    type Target = Queue;

    fn deref(&self) -> &Queue {
        &self.0
    }
}

impl ActionQueue {
    pub fn new(redis_pool: RedisPool) -> ActionQueue {
        let queue_name = match redis_pool.key_prefix() {
            Some(prefix) => format!("{}-{}", prefix, QUEUE_NAME),
            None => QUEUE_NAME.to_string(),
        };

        ActionQueue(Queue::new(redis_pool, queue_name, None, None, None))
    }
}

/// Create an action queue and a task to drain the Postgres staging table into the queue.
pub fn new_drain(
    queue: ActionQueue,
    db_pool: PostgresPool,
    redis_pool: RedisPool,
    shutdown: GracefulShutdownConsumer,
) -> Result<QueueStageDrain, Error> {
    let config = QueueStageDrainConfig {
        db_pool,
        redis_pool,
        drainer: QueueDrainer {},
        queue: Some(queue.0),
        shutdown,
    };

    QueueStageDrain::new(config)
}
