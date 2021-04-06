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
    close: oneshot::Sender<()>,
    join_handle: tokio::task::JoinHandle<()>,
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

        let join_handle = tokio::spawn(move || {
            drain.queue_task(
                config.db_pool,
                db_select_query,
                db_delete_query,
                config.jetstream_queue,
                config.nats_connection,
                close_rx,
                config.shutdown,
            )
        });

        Ok(QueueStageDrain {
            close: close_tx,
            join_handle,
        })
    }

    pub async fn close(&mut self) -> Result<(), Error> {
        let QueueStageDrain { close, join_handle } = self;
        close.send(()).ok();
        join_handle.await
    }
}

impl Drop for QueueStageDrain {
    fn drop(&mut self) {
        self.close()
    }
}

async fn drain_task(
    db_pool: VaultPostgresPool,
    db_select_query: String,
    db_delete_query: String,
    jetstream_queue: String,
    nats_connection: nats::Connection,
    close: oneshot::Receiver<()>,
    shutdown: GracefulShutdownConsumer,
) {
    loop {
        // TODO no unwrap
        let conn = db_pool.acquire().await.unwrap();
        let found_some = conn.transaction(|tx| {
            let db_select_query = &db_select_query;
            let db_delete_query = &db_delete_query;

            Box::pin(async move {
                let rows = sqlx::query(db_select_query.as_str()).fetch_all(tx).await?;

                if rows.is_empty() {
                    return Ok(false);
                }

                for row in rows {
                    let id = row.get(1);
                    let value = row.get(2);
                }

                return Ok(true);
            })
        });

        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(100)) => continue,
            _ = shutdown.wait_for_shutdown() => break,
            _ = close => break,
        }
    }
}
