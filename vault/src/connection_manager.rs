use async_trait::async_trait;
use deadpool::managed::RecycleError;
use hashicorp_vault::client::PostgresqlLogin;
use serde::de::DeserializeOwned;
use sqlx::{Connection, PgConnection};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock};
use tracing::{event, span, Level};

use crate::Error;
use graceful_shutdown::GracefulShutdownConsumer;

async fn refresh_loop<T: 'static + DeserializeOwned + Send + Sync>(
    manager: Arc<Manager<T>>,
    mut shutdown: GracefulShutdownConsumer,
    initial_renewable: Option<String>,
    initial_lease_duration: std::time::Duration,
) {
    let mut renew_lease_id = initial_renewable;
    let mut wait_time = tokio::time::Instant::now() + initial_lease_duration.div_f32(2.0);
    loop {
        tokio::select! {
            _ = tokio::time::sleep_until(wait_time) => {
                let lease_id = renew_lease_id.clone();
                let m = manager.clone();
                let result = tokio::task::spawn_blocking(move || {
                        m.refresh_auth(lease_id.as_ref().map(|l| l.as_str()))
                    })
                    .await
                    .unwrap();

                match result {
                    Ok((lease_id, lease_duration)) => {
                        renew_lease_id = lease_id;
                        wait_time = tokio::time::Instant::now() + lease_duration.div_f32(2.0);
                    }
                    Err(e) => {
                        // For now this is handled in the function itself
                        event!(Level::ERROR, error=?e, "Failed to refresh auth");
                    }
                }
            },
            true = shutdown.shutting_down() => {
                break;
            }
        }
    }
}

pub(crate) struct Manager<T: 'static + DeserializeOwned + Send + Sync> {
    connection_string: RwLock<String>,
    vault_client: crate::SharedVaultClient<T>,

    host: String,
    database: String,
    role: String,
}

impl<T: 'static + DeserializeOwned + Send + Sync> Manager<T> {
    pub(crate) fn new(
        vault_client: crate::SharedVaultClient<T>,
        shutdown: GracefulShutdownConsumer,
        host: String,
        database: String,
        role: String,
    ) -> Result<Arc<Manager<T>>, Error> {
        let manager = Manager {
            connection_string: RwLock::new(String::new()),
            vault_client,
            host,
            database,
            role,
        };

        let (renewable, duration) = manager.refresh_auth(None)?;

        let manager_ptr = Arc::new(manager);
        tokio::task::spawn(refresh_loop(
            manager_ptr.clone(),
            shutdown,
            renewable,
            duration,
        ));

        Ok(manager_ptr)
    }

    fn get_connection_string(&self, user: &str, password: &str) -> String {
        format!(
            "postgresql://{user}:{password}@{host}/{database}",
            user = user,
            password = password,
            host = self.host,
            database = self.database
        )
    }

    pub fn refresh_auth(
        &self,
        renew_lease_id: Option<&str>,
    ) -> Result<(Option<String>, std::time::Duration), Error> {
        // If the lease is renewable, then try that first.
        let span = span!(Level::INFO, "refreshing Postgres auth", role=%self.role );
        let _enter = span.enter();

        if let Some(lease_id) = renew_lease_id {
            event!(Level::INFO, "Refreshing renewable lease");
            match self
                .vault_client
                .read()
                .unwrap()
                .renew_lease(lease_id, None)
            {
                Ok(auth) => {
                    let renewable = auth.renewable.unwrap_or(false);
                    let renew_lease_id = if renewable { auth.lease_id } else { None };

                    let lease_duration = auth
                        .lease_duration
                        .map(|d| d.0)
                        .unwrap_or(std::time::Duration::new(3600, 0));
                    return Ok((renew_lease_id, lease_duration));
                }
                Err(e) => {
                    // It didn't work, so try getting a new role.
                    event!(
                        Level::ERROR,
                        role = %self.role,
                        error = %e,
                        "Failed to refresh lease"
                    );
                }
            }
        }

        event!(Level::INFO, "Fetching new credentials");
        let auth = self
            .vault_client
            .read()
            .unwrap()
            .get_secret_engine_creds::<PostgresqlLogin>("database", &self.role)?;

        let data = auth.data.as_ref().ok_or(Error::VaultNoDataError)?;

        let new_conn = self.get_connection_string(&data.username, &data.password);
        {
            let mut conn = self.connection_string.write().unwrap();
            *conn = new_conn;
        }

        let renewable = auth.renewable.unwrap_or(false);
        let renew_lease_id = if renewable { auth.lease_id } else { None };

        let lease_duration = auth
            .lease_duration
            .map(|d| d.0)
            .unwrap_or(std::time::Duration::new(3600, 0));

        event!(
            Level::INFO,
            renewable,
            ?lease_duration,
            "Got new credentials"
        );

        Ok((renew_lease_id, lease_duration))
    }
}

pub struct WrappedConnection {
    conn_str: String,
    pub conn: PgConnection,
}

impl Deref for WrappedConnection {
    type Target = PgConnection;
    fn deref(&self) -> &Self::Target {
        &self.conn
    }
}

impl DerefMut for WrappedConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.conn
    }
}

// #[async_trait]
// impl sqlx::Executor for WrappedConnection {
//     fn fetch_many(self, query ) {
//         self.conn.fetch_many(query)
//     }
// }

#[async_trait]
impl<T: DeserializeOwned + Send + Sync> deadpool::managed::Manager<WrappedConnection, Error>
    for Arc<Manager<T>>
{
    async fn create(&self) -> Result<WrappedConnection, Error> {
        let this_str = { self.connection_string.read().unwrap().clone() };
        let conn = sqlx::PgConnection::connect(&this_str).await?;
        Ok(WrappedConnection {
            conn_str: this_str,
            conn,
        })
    }

    async fn recycle(
        &self,
        connection: &mut WrappedConnection,
    ) -> deadpool::managed::RecycleResult<Error> {
        let stale = { connection.conn_str != *self.connection_string.read().unwrap() };
        if stale {
            return Err(RecycleError::Message("expired user".to_string()));
        }

        connection
            .conn
            .ping()
            .await
            .map_err(|e| RecycleError::Backend(Error::from(e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn connection_change() {}
}
