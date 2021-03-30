use std::{borrow::Cow, time::Duration};

use super::{connection_manager::WrappedConnection, VaultPostgresPool};
use crate::error::Error;

pub async fn serializable<F, T, E>(
    pool: &VaultPostgresPool<()>,
    retries: usize,
    mut run: F,
) -> Result<T, Error>
where
    F: FnMut(&mut sqlx::PgConnection) -> Result<T, E>,
    T: Send + Sync,
    E: Into<Error> + Send + Sync,
{
    let mut retried = 0;
    let mut sleep = Duration::from_millis(10);

    while retried <= retries {
        let mut conn = pool.acquire().await?;
        sqlx::query!("BEGIN").execute(&mut *conn).await?;
        sqlx::query!("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
            .execute(&mut *conn)
            .await?;
        let r = run(&mut conn.conn).map_err(|e| e.into());

        if let Err(Error::SqlError(sqlx::Error::Database(e))) = &r {
            if e.code().unwrap_or_else(|| Cow::from("")) == "serialization_failure" {
                retried += 1;
                tokio::time::sleep(sleep).await;
                sleep = sleep.mul_f32(2.0);
                continue;
            }
        }

        match r {
            Ok(value) => {
                sqlx::query!("COMMIT").execute(&mut *conn).await?;
                return Ok(value);
            }
            Err(e) => {
                sqlx::query!("ROLLBACK").execute(&mut *conn).await?;
                return Err(e);
            }
        }
    }

    Err(Error::SerializationFailure)
}
