use std::{borrow::Cow, ops::Deref};

use crate::{error::Error, Task};

use async_trait::async_trait;
use ergo_database::{object_id::*, PostgresPool, RedisPool};
use ergo_graceful_shutdown::GracefulShutdownConsumer;
use ergo_notifications::{Notification, NotificationManager, NotifyEvent};
use ergo_queues::{
    postgres_drain::{Drainer, QueueStageDrain, QueueStageDrainConfig},
    Job, JobId, Queue,
};
use sqlx::{Connection, Postgres, Transaction};
use uuid::Uuid;

use super::validate_input_payload;

struct QueueDrainer {}

#[async_trait]
impl Drainer for QueueDrainer {
    type Error = Error;

    fn lock_key(&self) -> i64 {
        79034890
    }

    async fn get(
        &self,
        tx: &mut Transaction<Postgres>,
    ) -> Result<Vec<(Cow<'static, str>, Job)>, Error> {
        let results = sqlx::query!(
            r##"SELECT event_queue_id,
                task_id as "task_id: TaskId",
                task_trigger_id as "task_trigger_id: TaskTriggerId",
                input_id as "input_id: InputId",
                inputs_log_id, payload
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

    QueueStageDrain::new(config).map_err(|e| e.into())
}

pub struct EnqueueInputOptions<'a> {
    /// If true, apply the input immediately instead of adding it to the queue.
    pub run_immediately: bool,
    /// If both `run_immediately` and `immediate_actions` are true, then run actions immediately
    /// instead of enqueueing them.
    pub immediate_actions: bool,
    pub pg: &'a PostgresPool,
    pub notifications: Option<NotificationManager>,
    pub org_id: OrgId,
    pub task_id: TaskId,
    pub task_name: String,
    pub input_id: InputId,
    pub task_trigger_id: TaskTriggerId,
    pub task_trigger_local_id: String,
    pub task_trigger_name: String,
    pub payload_schema: &'a serde_json::Value,
    pub payload: serde_json::Value,
}

pub async fn enqueue_input(options: EnqueueInputOptions<'_>) -> Result<Uuid, Error> {
    let EnqueueInputOptions {
        run_immediately,
        immediate_actions,
        pg,
        notifications,
        org_id,
        task_id,
        task_name,
        input_id,
        task_trigger_id,
        task_trigger_local_id,
        task_trigger_name,
        payload_schema,
        payload,
    } = options;

    validate_input_payload(&input_id, payload_schema, &payload)?;

    let input_arrival_id = Uuid::new_v4();

    let immediate_data = if run_immediately {
        Some((notifications.clone(), payload.clone()))
    } else {
        None
    };

    let mut conn = pg.acquire().await?;
    conn.transaction(|tx| {
        let input_id = input_id.clone();
        let task_id = task_id.clone();
        let task_trigger_id = task_trigger_id.clone();
        Box::pin(async move {
            if !run_immediately {
                sqlx::query!(
                    r##"INSERT INTO event_queue
            (task_id, input_id, task_trigger_id, inputs_log_id, payload) VALUES
            ($1, $2, $3, $4, $5)"##,
                    &task_id.0,
                    &input_id.0,
                    &task_trigger_id.0,
                    &input_arrival_id,
                    payload
                )
                .execute(&mut *tx)
                .await?;
            }

            sqlx::query!(
                r##"INSERT INTO inputs_log
        (inputs_log_id, task_trigger_id, task_id, task_trigger_local_id, status, payload)
        VALUES
        ($1, $2, $3, $4, 'pending', $5)"##,
                &input_arrival_id,
                &task_trigger_id.0,
                &task_id.0,
                &task_trigger_local_id,
                &payload
            )
            .execute(&mut *tx)
            .await?;

            if let Some(notify) = notifications {
                let notification = Notification {
                    task_id,
                    local_id: task_trigger_local_id,
                    local_object_id: Some(task_trigger_id.into_inner()),
                    local_object_name: task_trigger_name,
                    error: None,
                    event: NotifyEvent::InputArrived,
                    task_name,
                    log_id: Some(input_arrival_id),
                    payload: Some(payload),
                };
                notify.notify(&mut *tx, &org_id, notification).await?;
            }

            Ok::<(), Error>(())
        })
    })
    .await?;

    if let Some((notifications, payload)) = immediate_data {
        Task::apply_input(
            pg,
            notifications,
            task_id,
            input_id,
            task_trigger_id,
            input_arrival_id,
            payload,
            immediate_actions,
        )
        .await?;
    }

    Ok(input_arrival_id)
}
