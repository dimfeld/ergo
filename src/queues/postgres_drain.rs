use std::{sync::Arc, time::Duration};

use sqlx::Connection;
use tokio::sync::oneshot;

use crate::{
    database::VaultPostgresPool, error::Error, graceful_shutdown::GracefulShutdownConsumer,
};

pub struct QueueStageDrainConfig<'a> {
    pub db_pool: VaultPostgresPool,
    pub db_table: &'a str,
    pub db_id_column: &'a str,

    pub jetstream_queue: String,
    pub nats_connection: nats::Connection,
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
            "SELECT {id_column}, row_to_json(t) FROM {db_table} t
            ORDER BY {id_column} LIMIT {batch_size}",
            id_column = &config.db_id_column,
            db_table = &config.db_table,
            batch_size = batch_size
        );

        let db_delete_query = format!(
            "DELETE FROM {db_table}
            WHERE {id_column} < $1",
            db_table = &config.db_table,
            id_column = &config.db_id_column,
        );

        let (close_tx, close_rx) = tokio::sync::oneshot::channel::<()>();

        let QueueStageDrainConfig {
            db_pool,
            queue,
            shutdown,
            ..
        } = config;

        let join_handle = tokio::spawn(drain_task(
            db_pool,
            db_select_query,
            db_delete_query,
            queue,
            close_rx,
            shutdown,
        ));

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

async fn drain_task(
    db_pool: PostgresPool,
    db_select_query: String,
    db_delete_query: String,
    queue: Queue,
    close: oneshot::Receiver<()>,
    mut shutdown: GracefulShutdownConsumer,
) {
    tokio::pin!(close);
    let shutdown_waiter = shutdown.wait_for_shutdown();
    tokio::pin!(shutdown_waiter);

    loop {
        let db_select_query = db_select_query.clone();
        let db_delete_query = db_delete_query.clone();

        // TODO no unwrap
        let mut conn = db_pool.acquire().await.unwrap();
        let found_some = conn
            .transaction(move |tx| {
                Box::pin(async move {
                    let lock_result = sqlx::query("SELECT pg_try_advisory_xact_lock(7893478934);")
                        .fetch_one(&mut *tx)
                        .await?;
                    let acquired_lock: bool = lock_result.get(0);
                    if acquired_lock == false {
                        // Something else has the lock, so just exit and try again after a sleep.
                        return Ok(false);
                    }

                    let rows = sqlx::query(&db_select_query).fetch_all(&mut *tx).await?;

                    if rows.is_empty() {
                        return Ok(false);
                    }

                    let queue_items = rows
                        .into_iter()
                        .map(|row| {
                            (
                                row.get::<String, usize>(0),
                                row.get::<serde_json::Value, usize>(1),
                            )
                        })
                        .collect::<Vec<_>>();

                    // TODO enqueue items

                    sqlx::query(&db_delete_query).execute(&mut *tx).await?;

                    return Ok::<bool, Error>(true);
                })
            })
            .await;
        drop(conn);

        match found_some {
            Ok(true) => {
                continue;
            }
            Ok(false) => {}
            Err(e) => {
                event!(Level::ERROR, error=?e, "Error draining job queue");
            }
        };

        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(100)) => continue,
            _ = &mut shutdown_waiter => break,
            _ = &mut close => break,
        }
    }
}
