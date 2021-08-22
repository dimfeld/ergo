use futures::future::BoxFuture;
use std::{borrow::Cow, time::Duration};

use sqlx::{error::DatabaseError, Connection, PgConnection, Postgres};

use crate::Error;

pub trait TryIntoSqlxError: Sized {
    /// If this error type contains a sqlx::Error, return it.
    /// Otherwise return Err(self).
    fn try_into_sqlx_error(self) -> Result<sqlx::Error, Self>;
}

impl TryIntoSqlxError for sqlx::Error {
    fn try_into_sqlx_error(self) -> Result<sqlx::Error, Self> {
        Ok(self)
    }
}

pub fn serializable<F, T, E>(
    conn: &mut PgConnection,
    retries: usize,
    run: F,
) -> BoxFuture<'_, Result<T, E>>
where
    for<'c> F: Fn(&'c mut sqlx::Transaction<'_, Postgres>) -> BoxFuture<'c, Result<T, E>>
        + 'static
        + Send
        + Sync,
    T: Send,
    E: From<Error> + From<sqlx::Error> + TryIntoSqlxError + Send,
{
    Box::pin(async move {
        let mut retried = 0;
        let mut sleep = Duration::from_millis(10);

        while retried <= retries {
            let mut tx = conn.begin().await?;
            sqlx::query!("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
                .execute(&mut tx)
                .await?;

            match run(&mut tx).await {
                Ok(value) => {
                    tx.commit().await?;
                    return Ok(value);
                }

                Err(e) => match e.try_into_sqlx_error() {
                    Ok(sqlx::Error::Database(sql_error)) => {
                        if sql_error.code().unwrap_or_else(|| Cow::from("")) == "40001" {
                            // It's a serialization error
                            retried += 1;
                            tokio::time::sleep(sleep).await;
                            sleep = sleep.mul_f32(2.0);
                            continue;
                        } else {
                            tx.rollback().await?;
                            return Err(sqlx::Error::Database(sql_error).into());
                        }
                    }
                    Ok(e) => {
                        // It was an sqlx::error, but not a Database error specifically.
                        tx.rollback().await?;
                        return Err(e.into());
                    }
                    Err(e) => {
                        tx.rollback().await?;
                        return Err(e);
                    }
                },
            };
        }

        Err(Error::SerializationFailure.into())
    })
}
