use std::{borrow::Cow, pin::Pin, sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use rand::Rng;
use sqlx::{Connection, Postgres, Row};
use tokio::{sync::oneshot, task::JoinHandle};
use tracing::{event, Level};

use super::{Job, Queue};
use crate::{database::PostgresPool, error::Error, graceful_shutdown::GracefulShutdownConsumer};

pub struct QueueStageDrainConfig<'a> {
    pub db_pool: PostgresPool,
    pub db_table: &'a str,

    pub queue: Queue,
    pub shutdown: GracefulShutdownConsumer,

    /// The number of items to drain at once. Defaults to 50.
    pub drain_batch_size: Option<usize>,
}

/// This implements the drain stage of a transactionally-staged job drain, as described
/// at https://brandur.org/job-drain.
pub struct QueueStageDrain {
    close: Option<oneshot::Sender<()>>,
    join_handle: Option<tokio::task::JoinHandle<()>>,
}

impl QueueStageDrain {
    pub fn new(config: QueueStageDrainConfig) -> Result<Self, Error> {
        let batch_size = config.drain_batch_size.unwrap_or(50);
        let db_select_query = format!(
            "SELECT id, max_retries, timeout, run_at, data FROM {db_table} t
            ORDER BY id LIMIT {batch_size}",
            db_table = &config.db_table,
            batch_size = batch_size
        );

        let db_delete_query = format!(
            "DELETE FROM {db_table}
            WHERE id < $1",
            db_table = &config.db_table,
        );

        let (close_tx, close_rx) = tokio::sync::oneshot::channel::<()>();

        let QueueStageDrainConfig {
            db_pool,
            queue,
            shutdown,
            ..
        } = config;

        let drain = StageDrainTask {
            db_pool,
            db_select_query,
            db_delete_query,
            queue,
            close: close_rx,
            shutdown,
        };

        let join_handle = tokio::spawn(drain.start());

        Ok(QueueStageDrain {
            close: Some(close_tx),
            join_handle: Some(join_handle),
        })
    }

    // Close without consuming the object. Needed for drop implementation.
    fn close_internal(&mut self) -> Option<JoinHandle<()>> {
        if let Some(close) = self.close.take() {
            close.send(()).ok();
        }

        self.join_handle.take()
    }

    pub fn close(mut self) -> JoinHandle<()> {
        self.close_internal().unwrap()
    }
}

impl Drop for QueueStageDrain {
    fn drop(&mut self) {
        self.close_internal();
    }
}

struct StageDrainTask {
    db_pool: PostgresPool,
    db_select_query: String,
    db_delete_query: String,
    queue: Queue,
    close: oneshot::Receiver<()>,
    shutdown: GracefulShutdownConsumer,
}

const INITIAL_SLEEP: std::time::Duration = Duration::from_millis(25);
const MAX_SLEEP: std::time::Duration = Duration::from_secs(5);

/// Return the initial sleep value, perturbed a bit to prevent lockstep
/// exponential retry.
fn initial_sleep_value() -> std::time::Duration {
    let perturb = (rand::random::<f64>() - 0.5) * 5000.0;
    INITIAL_SLEEP + Duration::from_micros(perturb as u64)
}

impl StageDrainTask {
    async fn start(mut self) {
        let mut shutdown_waiter = self.shutdown.clone();
        let mut sleep_duration = initial_sleep_value();

        loop {
            match self.try_drain().await {
                Ok(true) => {
                    // We got some rows, so reset the sleep duration and run again immediately.
                    sleep_duration = initial_sleep_value();
                    continue;
                }
                Ok(false) => {
                    // No rows, so just fall through to the delay
                }
                Err(e) => {
                    event!(Level::ERROR, error=?e, "Error draining job queue");
                }
            };

            sleep_duration = sleep_duration.mul_f64(2.0).min(MAX_SLEEP);
            tokio::select! {
                _ = tokio::time::sleep(sleep_duration) => continue,
                _ = shutdown_waiter.wait_for_shutdown() => break,
                _ = &mut self.close => break,
            }
        }
    }

    async fn try_drain(&mut self) -> Result<bool, Error> {
        let mut conn = self.db_pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let lock_result = sqlx::query("SELECT pg_try_advisory_xact_lock(7893478934)")
            .fetch_one(&mut tx)
            .await?;
        let acquired_lock: bool = lock_result.get(0);
        if acquired_lock == false {
            // Something else has the lock, so just exit and try again after a sleep.
            return Ok(false);
        }

        let rows = sqlx::query(&self.db_select_query)
            .fetch_all(&mut tx)
            .await?;

        if rows.is_empty() {
            return Ok(false);
        }

        let queue_items = rows
            .into_iter()
            .map(|row| {
                let max_retries: Option<i32> = row.get(1);
                let timeout: Option<u32> = row.get(2);
                let payload = row.get::<String, usize>(4);

                Job {
                    // TODO  Job should generate its own UUID
                    id: String::from(""),
                    max_retries: max_retries.map(|i| if i < 0 { 0 as u32 } else { i as u32 }),
                    timeout: timeout.map(|t| Duration::from_millis(t as u64)),
                    retry_backoff: None,
                    run_at: row.get::<Option<DateTime<Utc>>, usize>(3),
                    payload: Cow::Owned(Vec::from(payload)),
                }
            })
            .collect::<Vec<_>>();

        self.queue.enqueue_multiple(queue_items.as_slice()).await?;

        sqlx::query(&self.db_delete_query).execute(&mut tx).await?;

        return Ok::<bool, Error>(true);
    }
}
