use futures::future::BoxFuture;
use std::{borrow::Cow, time::Duration};

use sqlx::{Connection, PgConnection, Postgres};

use crate::error::Error;

pub fn serializable<F, T, E>(
    conn: &mut PgConnection,
    retries: usize,
    run: F,
) -> BoxFuture<'_, Result<T, Error>>
where
    for<'c> F: Fn(&'c mut sqlx::Transaction<'_, Postgres>) -> BoxFuture<'c, Result<T, E>>
        + 'static
        + Send
        + Sync,
    T: Send,
    E: Into<Error> + Send,
{
    Box::pin(async move {
        let mut retried = 0;
        let mut sleep = Duration::from_millis(10);

        while retried <= retries {
            let mut tx = conn.begin().await?;
            sqlx::query!("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
                .execute(&mut tx)
                .await?;
            let r = run(&mut tx).await.map_err(|e| e.into());

            let is_serialization_error = {
                if let Err(Error::SqlError(sqlx::Error::Database(e))) = &r {
                    // Check for serialization error code
                    e.code().unwrap_or_else(|| Cow::from("")) == "40001"
                } else {
                    false
                }
            };

            if is_serialization_error {
                retried += 1;
                tokio::time::sleep(sleep).await;
                sleep = sleep.mul_f32(2.0);
                continue;
            }

            match r {
                Ok(value) => {
                    tx.commit().await?;
                    return Ok(value);
                }
                Err(e) => {
                    tx.rollback().await?;
                    return Err(e);
                }
            }
        }

        Err(Error::SerializationFailure)
    })
}
