use async_trait::async_trait;
use deadpool::managed::RecycleError;
use derivative::Derivative;
use hashicorp_vault::client::{PostgresqlLogin, VaultClient, VaultResponse};
use serde::de::DeserializeOwned;
use sqlx::{Connection, PgConnection};
use std::sync::{atomic::Ordering, Arc, RwLock};
use std::{
    ops::{Deref, DerefMut},
    sync::atomic::AtomicU64,
};
use tracing::{event, span, Level};

use crate::Error;
use graceful_shutdown::GracefulShutdownConsumer;

pub trait PostgresAuthRenewer: 'static + Send + Sync {
    fn renew_lease(&self, lease_id: impl Into<String>) -> Result<VaultResponse<()>, Error>;
    fn get_lease(&self, role: &str) -> Result<VaultResponse<PostgresqlLogin>, Error>;
}

impl<T: 'static + DeserializeOwned + Send + Sync> PostgresAuthRenewer for VaultClient<T> {
    fn renew_lease(&self, lease_id: impl Into<String>) -> Result<VaultResponse<()>, Error> {
        VaultClient::renew_lease(self, lease_id, None).map_err(Error::from)
    }

    fn get_lease(&self, role: &str) -> Result<VaultResponse<PostgresqlLogin>, Error> {
        self.get_secret_engine_creds::<PostgresqlLogin>("database", role)
            .map_err(Error::from)
    }
}

pub type SharedRenewer<T> = Arc<RwLock<T>>;

async fn refresh_loop<T: PostgresAuthRenewer>(
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
                    .map_err(Error::from)
                    .and_then(|r| r);

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
            _ = shutdown.wait_for_shutdown() => {
                break;
            }
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub(crate) struct Manager<RENEWER: PostgresAuthRenewer> {
    connection_string: RwLock<String>,
    #[derivative(Debug = "ignore")]
    vault_client: Arc<RwLock<RENEWER>>,

    host: String,
    database: String,
    role: String,

    pub(crate) update_successes: AtomicU64,
    pub(crate) update_failures: AtomicU64,
    pub(crate) renew_successes: AtomicU64,
    pub(crate) renew_failures: AtomicU64,
}

impl<RENEWER: PostgresAuthRenewer> Manager<RENEWER> {
    pub(crate) fn new(
        vault_client: SharedRenewer<RENEWER>,
        shutdown: GracefulShutdownConsumer,
        host: String,
        database: String,
        role: String,
    ) -> Result<Arc<Manager<RENEWER>>, Error> {
        let manager = Manager {
            connection_string: RwLock::new(String::new()),
            vault_client,
            host,
            database,
            role,

            update_successes: AtomicU64::new(0),
            update_failures: AtomicU64::new(0),
            renew_successes: AtomicU64::new(0),
            renew_failures: AtomicU64::new(0),
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
            match self.vault_client.read().unwrap().renew_lease(lease_id) {
                Ok(auth) => {
                    self.renew_successes.fetch_add(1, Ordering::Relaxed);
                    let renewable = auth.renewable.unwrap_or(false);
                    let renew_lease_id = if renewable { auth.lease_id } else { None };

                    let lease_duration = auth
                        .lease_duration
                        .map(|d| d.0)
                        .unwrap_or(std::time::Duration::new(3600, 0));
                    return Ok((renew_lease_id, lease_duration));
                }
                Err(e) => {
                    self.renew_failures.fetch_add(1, Ordering::Relaxed);
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
        let auth = match self.vault_client.read().unwrap().get_lease(&self.role) {
            Ok(data) => {
                self.update_successes.fetch_add(1, Ordering::Relaxed);
                data
            }
            Err(e) => {
                self.update_failures.fetch_add(1, Ordering::Relaxed);
                return Err(e);
            }
        };

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

#[derive(Debug)]
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
impl<T: PostgresAuthRenewer> deadpool::managed::Manager<WrappedConnection, Error>
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
    use super::*;
    use hashicorp_vault::client::VaultDuration;
    use std::time::Duration;

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    struct MockData {
        renew_count: usize,
        get_count: usize,
    }

    impl MockData {
        fn new() -> MockData {
            MockData {
                renew_count: 0,
                get_count: 0,
            }
        }
    }

    struct MockVaultClient {
        lease_id: String,
        lease_duration: Duration,
        renewable: bool,
        role: String,

        postgres_user: String,
        postgres_password: String,

        counts: RwLock<MockData>,
    }

    impl MockVaultClient {
        fn check_counts(&self, expected_get_count: usize, expected_renew_count: usize) {
            let counts = self.counts.read().unwrap();
            assert_eq!(
                counts.get_count, expected_get_count,
                "get count expected {} saw {}",
                expected_get_count, counts.get_count
            );
            assert_eq!(
                counts.renew_count, expected_renew_count,
                "renew count expected {} saw {}",
                expected_renew_count, counts.renew_count
            );
        }
    }

    impl PostgresAuthRenewer for MockVaultClient {
        fn renew_lease(&self, lease_id: impl Into<String>) -> Result<VaultResponse<()>, Error> {
            {
                let mut counts = self.counts.write().unwrap();
                counts.renew_count += 1;
            }

            if self.renewable && lease_id.into() == self.lease_id {
                Ok(VaultResponse {
                    request_id: String::new(),
                    lease_id: Some(self.lease_id.clone()),
                    renewable: Some(self.renewable),
                    lease_duration: Some(VaultDuration(self.lease_duration)),
                    data: None,
                    warnings: None,
                    auth: None,
                    wrap_info: None,
                })
            } else {
                Err(Error::VaultNoDataError)
            }
        }

        fn get_lease(&self, role: &str) -> Result<VaultResponse<PostgresqlLogin>, Error> {
            assert_eq!(role, self.role);

            {
                let mut counts = self.counts.write().unwrap();
                counts.get_count += 1;
            }

            Ok(VaultResponse {
                request_id: String::new(),
                lease_id: Some(self.lease_id.clone()),
                renewable: Some(self.renewable),
                lease_duration: Some(VaultDuration(self.lease_duration)),
                data: Some(PostgresqlLogin {
                    username: self.postgres_user.clone(),
                    password: self.postgres_password.clone(),
                }),
                warnings: None,
                auth: None,
                wrap_info: None,
            })
        }
    }

    #[tokio::test(start_paused = true)]
    async fn renews_role_lease() {
        let shutdown = graceful_shutdown::GracefulShutdown::new();
        let vault_client = Arc::new(RwLock::new(MockVaultClient {
            lease_id: "l1".to_string(),
            lease_duration: Duration::from_secs(300),
            renewable: true,
            role: "dbrole".to_string(),
            postgres_user: "username".to_string(),
            postgres_password: "pwd".to_string(),

            counts: RwLock::new(MockData::new()),
        }));

        Manager::new(
            vault_client.clone(),
            shutdown.consumer(),
            "host".to_string(),
            "database".to_string(),
            "dbrole".to_string(),
        )
        .unwrap();

        vault_client.read().unwrap().check_counts(1, 0);

        tokio::time::sleep(Duration::from_secs(140)).await;
        vault_client.read().unwrap().check_counts(1, 0);
        tokio::time::sleep(Duration::from_secs(20)).await;

        // This is horrible, but accounts for the fact that there isn't a good way to synchronize
        // with the spawn_blocking task that does the refresh right now.
        const MAX_CHECKS: usize = 10;
        let mut checks = 0;
        while checks < MAX_CHECKS {
            tokio::task::yield_now().await;
            if vault_client
                .read()
                .unwrap()
                .counts
                .read()
                .unwrap()
                .renew_count
                > 0
            {
                break;
            } else {
                checks += 1;
                std::thread::sleep(Duration::from_millis(5));
            }
        }

        if checks == MAX_CHECKS {
            panic!("Timed out waiting for renew to happen");
        }

        vault_client.read().unwrap().check_counts(1, 1);
    }

    #[tokio::test(start_paused = true)]
    async fn updates_nonrenewable_lease() {
        let shutdown = graceful_shutdown::GracefulShutdown::new();
        let vault_client = Arc::new(RwLock::new(MockVaultClient {
            lease_id: "l1".to_string(),
            lease_duration: Duration::from_secs(300),
            renewable: false,
            role: "dbrole".to_string(),
            postgres_user: "username".to_string(),
            postgres_password: "pwd".to_string(),

            counts: RwLock::new(MockData::new()),
        }));

        Manager::new(
            vault_client.clone(),
            shutdown.consumer(),
            "host".to_string(),
            "database".to_string(),
            "dbrole".to_string(),
        )
        .unwrap();

        vault_client.read().unwrap().check_counts(1, 0);

        tokio::time::sleep(Duration::from_secs(140)).await;
        vault_client.read().unwrap().check_counts(1, 0);
        tokio::time::sleep(Duration::from_secs(20)).await;

        // This is horrible, but accounts for the fact that there isn't a good way to synchronize
        // with the spawn_blocking task that does the refresh right now.
        const MAX_CHECKS: usize = 10;
        let mut checks = 0;
        while checks < MAX_CHECKS {
            tokio::task::yield_now().await;
            if vault_client
                .read()
                .unwrap()
                .counts
                .read()
                .unwrap()
                .get_count
                > 1
            {
                break;
            } else {
                checks += 1;
                std::thread::sleep(Duration::from_millis(5));
            }
        }

        if checks == MAX_CHECKS {
            panic!("Timed out waiting for renew to happen");
        }

        vault_client.read().unwrap().check_counts(2, 0);
    }
}
