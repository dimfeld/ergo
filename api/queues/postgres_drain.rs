use std::{borrow::Cow, time::Duration};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ergo_database::{PostgresPool, RedisPool};
use fxhash::FxHashMap;
use serde::Serialize;
use sqlx::{Connection, Postgres, Row, Transaction};
use tokio::{
    sync::{oneshot, watch},
    task::JoinHandle,
};
use tracing::{event, Level};

use super::{Job, Queue};
use crate::error::Error;
use ergo_graceful_shutdown::GracefulShutdownConsumer;

#[async_trait]
pub trait Drainer: Send + Sync {
    /// An advisory lock key to use when draining
    fn lock_key(&self) -> i64;

    /// Retrieve and delete jobs from the table.
    async fn get(
        &'_ self,
        tx: &mut Transaction<Postgres>,
    ) -> Result<Vec<(Cow<'static, str>, Job<'_>)>, Error>;
}

#[derive(Clone, Debug, Serialize)]
pub struct QueueStageDrainStats {
    pub drained: usize,
    pub last_drain: DateTime<Utc>,
    pub last_check: DateTime<Utc>,
}

pub struct QueueStageDrainConfig<D: Drainer + 'static> {
    pub db_pool: PostgresPool,
    pub redis_pool: RedisPool,
    pub drainer: D,

    /// Preinitialize with a queue when you already have the queue object.
    pub queue: Option<Queue>,
    pub shutdown: GracefulShutdownConsumer,
}

/// This implements the drain of a transactionally-staged job drain, as described
/// at <https://brandur.org/job-drain>.
pub struct QueueStageDrain {
    close: Option<oneshot::Sender<()>>,
    join_handle: Option<tokio::task::JoinHandle<()>>,

    pub stats: watch::Receiver<QueueStageDrainStats>,
}

impl QueueStageDrain {
    pub fn new<D: Drainer + 'static>(config: QueueStageDrainConfig<D>) -> Result<Self, Error> {
        let (close_tx, close_rx) = tokio::sync::oneshot::channel::<()>();

        let QueueStageDrainConfig {
            db_pool,
            redis_pool,
            queue,
            shutdown,
            drainer,
            ..
        } = config;

        let now = Utc::now();

        let (stats_tx, stats_rx) = watch::channel(QueueStageDrainStats {
            drained: 0,
            last_drain: now,
            last_check: now,
        });

        let drain = StageDrainTask {
            db_pool,
            redis_pool,
            queues: queue
                .into_iter()
                .map(|q| (q.name().to_string(), q))
                .collect::<_>(),
            drainer,
            close: close_rx,
            stats_tx,
            stats: QueueStageDrainStats {
                drained: 0,
                last_drain: now,
                last_check: now,
            },
            shutdown,
        };

        let join_handle = tokio::spawn(drain.start());

        Ok(QueueStageDrain {
            close: Some(close_tx),
            join_handle: Some(join_handle),
            stats: stats_rx,
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
    redis_pool: RedisPool,
    queues: FxHashMap<String, Queue>,
    drainer: D,
    close: oneshot::Receiver<()>,
    stats_tx: watch::Sender<QueueStageDrainStats>,
    shutdown: GracefulShutdownConsumer,

    stats: QueueStageDrainStats,
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

            self.stats_tx.send(self.stats.clone()).ok();

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
        let lock_result = sqlx::query(&format!(
            "SELECT pg_try_advisory_xact_lock({})",
            self.drainer.lock_key()
        ))
        .fetch_one(&mut tx)
        .await?;
        let acquired_lock: bool = lock_result.get(0);
        if acquired_lock == false {
            // Something else has the lock, so just exit and try again after a sleep.
            return Ok(false);
        }

        let now = Utc::now();
        self.stats.last_check = now;

        let jobs = self.drainer.get(&mut tx).await?;

        if jobs.is_empty() {
            return Ok(false);
        }

        self.stats.last_drain = now;

        for (queue_name, job) in &jobs {
            event!(Level::INFO, queue=%queue_name, ?job, "Enqueueing job");
            let queue = match self.queues.get(queue_name.as_ref()) {
                Some(q) => q,
                None => {
                    self.queues.insert(
                        queue_name.to_string(),
                        Queue::new(
                            self.redis_pool.clone(),
                            queue_name.to_string(),
                            None,
                            None,
                            None,
                        ),
                    );

                    self.queues.get(queue_name.as_ref()).unwrap()
                }
            };

            queue.enqueue(job).await?;
        }
        tx.commit().await?;

        return Ok::<bool, Error>(true);
    }
}
