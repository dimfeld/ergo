use std::{borrow::Cow, pin::Pin, sync::Arc, time::Duration};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rand::Rng;
use sqlx::{Connection, Postgres, Row, Transaction};
use tokio::{sync::oneshot, task::JoinHandle};
use tracing::{event, Level};

use super::{Job, JobId, Queue};
use crate::{database::PostgresPool, error::Error, graceful_shutdown::GracefulShutdownConsumer};

#[async_trait]
pub trait Drainer: Send + Sync {
    /// Retrieve and delete jobs from the table.
    async fn get(&'_ self, tx: &mut Transaction<Postgres>) -> Result<Vec<Job<'_>>, Error>;
}

pub struct QueueStageDrainConfig<D: Drainer + 'static> {
    pub db_pool: PostgresPool,
    pub drainer: D,

    pub queue: Queue,
    pub shutdown: GracefulShutdownConsumer,
}

/// This implements the drain of a transactionally-staged job drain, as described
/// at https://brandur.org/job-drain.
pub struct QueueStageDrain {
    close: Option<oneshot::Sender<()>>,
    join_handle: Option<tokio::task::JoinHandle<()>>,
}

impl QueueStageDrain {
    pub fn new<D: Drainer + 'static>(config: QueueStageDrainConfig<D>) -> Result<Self, Error> {
        let (close_tx, close_rx) = tokio::sync::oneshot::channel::<()>();

        let QueueStageDrainConfig {
            db_pool,
            queue,
            shutdown,
            drainer,
            ..
        } = config;

        let drain = StageDrainTask {
            db_pool,
            queue,
            drainer,
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

struct StageDrainTask<D: Drainer> {
    db_pool: PostgresPool,
    queue: Queue,
    drainer: D,
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

impl<D: Drainer> StageDrainTask<D> {
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

        let jobs = self.drainer.get(&mut tx).await?;

        if jobs.is_empty() {
            return Ok(false);
        }

        self.queue.enqueue_multiple(jobs.as_slice()).await?;

        return Ok::<bool, Error>(true);
    }
}
