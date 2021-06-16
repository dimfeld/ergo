use async_trait::async_trait;
use deadpool::managed::RecycleError;
use hashicorp_vault::client::{PostgresqlLogin, VaultClient, VaultDuration, VaultResponse};
use log::LevelFilter;
use serde::{de::DeserializeOwned, Serialize};
use sqlx::{postgres::PgConnectOptions, ConnectOptions, Connection, PgConnection};
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::Arc,
};
use tokio::sync::RwLock;
use tracing::{event, instrument, Level};

use super::{Error, VaultPostgresPoolAuth};
use crate::graceful_shutdown::GracefulShutdownConsumer;

#[async_trait]
pub trait PostgresAuthRenewer: 'static + Send + Sync + Debug {
    async fn renew_lease(&self, lease_id: &str) -> Result<VaultResponse<()>, Error>;
    async fn get_lease(&self, role: &str) -> Result<VaultResponse<PostgresqlLogin>, Error>;
}

#[async_trait]
impl<T: 'static + DeserializeOwned + Send + Sync + Debug> PostgresAuthRenewer
    for RwLock<VaultClient<T>>
{
    async fn renew_lease(&self, lease_id: &str) -> Result<VaultResponse<()>, Error> {
        self.read()
            .await
            .renew_lease(lease_id, None)
            .await
            .map_err(Error::from)
    }

    async fn get_lease(&self, role: &str) -> Result<VaultResponse<PostgresqlLogin>, Error> {
        self.read()
            .await
            .get_secret_engine_creds::<PostgresqlLogin>("database", role)
            .await
            .map_err(Error::from)
    }
}

pub type SharedRenewer<T> = Arc<RwLock<T>>;

async fn refresh_loop(
    manager: Arc<ManagerInner>,
    mut shutdown: GracefulShutdownConsumer,
    initial_renewable: Option<String>,
    initial_lease_duration: std::time::Duration,
) {
    if manager.renewer.is_none() {
        panic!("refresh_loop started but renewer is None");
    }

    let mut renew_lease_id = initial_renewable;
    let mut wait_time = tokio::time::Instant::now() + initial_lease_duration.div_f32(2.0);
    loop {
        tokio::select! {
            _ = tokio::time::sleep_until(wait_time) => {
                let lease_id = renew_lease_id.clone();
                let m = manager.clone();
                let result =
                    m.refresh_auth(lease_id.as_ref().map(|l| l.as_str()))
                    .await
                    .map_err(Error::from);

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

#[derive(PartialEq, Serialize, Clone, Debug)]
pub struct ManagerStats {
    pub update_successes: u64,
    pub update_failures: u64,
    pub renew_successes: u64,
    pub renew_failures: u64,
}

#[derive(Debug)]
pub struct Manager(pub(super) Arc<ManagerInner>);

#[derive(Debug)]
pub(crate) struct ManagerInner {
    creds: std::sync::RwLock<(String, String)>,
    renewer: Option<Arc<dyn PostgresAuthRenewer>>,

    host: String,
    port: u16,
    database: String,
    role: String,

    stats_sender: tokio::sync::watch::Sender<ManagerStats>,
    pub stats: tokio::sync::watch::Receiver<ManagerStats>,
}

impl Manager {
    pub(crate) async fn new(
        auth_method: VaultPostgresPoolAuth,
        shutdown: GracefulShutdownConsumer,
        host: String,
        port: u16,
        database: String,
    ) -> Result<Manager, Error> {
        let (stats_sender, stats_receiver) = tokio::sync::watch::channel(ManagerStats {
            update_successes: 0,
            update_failures: 0,
            renew_successes: 0,
            renew_failures: 0,
        });

        let (initial_credentials, renewer, role): (
            (String, String),
            Option<Arc<dyn PostgresAuthRenewer>>,
            String,
        ) = match auth_method {
            VaultPostgresPoolAuth::Vault { client, role } => {
                ((String::new(), String::new()), Some(client), role)
            }
            VaultPostgresPoolAuth::Password { username, password } => {
                ((username, password), None, String::new())
            }
        };

        let manager = Arc::new(ManagerInner {
            creds: std::sync::RwLock::new(initial_credentials),
            renewer,
            host,
            port,
            database,
            role,

            stats_sender,
            stats: stats_receiver,
        });

        if manager.renewer.is_some() {
            let (renewable, duration) = manager.refresh_auth(None).await?;
            tokio::task::spawn(refresh_loop(manager.clone(), shutdown, renewable, duration));
        }

        Ok(Manager(manager))
    }
}

impl ManagerInner {
    #[instrument(level="info", name="refreshing Postgres auth", fields(role=%self.role), skip(self))]
    async fn refresh_auth(
        &self,
        renew_lease_id: Option<&str>,
    ) -> Result<(Option<String>, std::time::Duration), Error> {
        let mut stats = { self.stats_sender.borrow().clone() };
        let renewer = self
            .renewer
            .as_ref()
            .expect("refresh_auth called without renewer");

        if let Some(lease_id) = renew_lease_id {
            event!(Level::INFO, "Refreshing renewable lease");
            match renewer.renew_lease(lease_id).await {
                Ok(auth) => {
                    stats.renew_successes += 1;
                    self.stats_sender.send(stats).ok();
                    let renewable = auth.renewable.unwrap_or(false);
                    let renew_lease_id = if renewable { auth.lease_id } else { None };

                    let lease_duration = auth
                        .lease_duration
                        .map(|d| d.0)
                        .unwrap_or(std::time::Duration::new(3600, 0));
                    return Ok((renew_lease_id, lease_duration));
                }
                Err(e) => {
                    stats.renew_failures += 1;
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
        let mut auth = match renewer.get_lease(&self.role).await {
            Ok(data) => {
                stats.update_successes += 1;
                self.stats_sender.send(stats).ok();
                data
            }
            Err(e) => {
                stats.update_failures += 1;
                self.stats_sender.send(stats).ok();
                return Err(e);
            }
        };

        let data = auth.data.take().ok_or(Error::VaultNoDataError)?;
        {
            let mut creds = self.creds.write().unwrap();
            *creds = (data.username, data.password)
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
    username: String,
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
impl deadpool::managed::Manager for Manager {
    type Type = WrappedConnection;
    type Error = Error;

    async fn create(&self) -> Result<WrappedConnection, Error> {
        let (username, password) = { self.0.creds.read().unwrap().clone() };
        let conn = PgConnectOptions::new()
            .username(&username)
            .password(&password)
            .host(&self.0.host)
            .port(self.0.port)
            .database(&self.0.database)
            .log_statements(LevelFilter::Debug)
            .connect()
            .await?;

        Ok(WrappedConnection { username, conn })
    }

    async fn recycle(
        &self,
        connection: &mut WrappedConnection,
    ) -> deadpool::managed::RecycleResult<Error> {
        let stale = { connection.username != *self.0.creds.read().unwrap().0 };
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
    use super::{super::VaultPostgresPoolAuth, *};
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

    #[derive(Debug)]
    struct MockVaultClient {
        lease_id: String,
        lease_duration: Duration,
        renewable: bool,
        role: String,

        postgres_user: String,
        postgres_password: String,

        counts: std::sync::RwLock<MockData>,
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

    #[async_trait]
    impl PostgresAuthRenewer for RwLock<MockVaultClient> {
        async fn renew_lease(&self, lease_id: &str) -> Result<VaultResponse<()>, Error> {
            {
                self.write().await.counts.write().unwrap().renew_count += 1;
            }

            let client = self.read().await;
            if client.renewable && lease_id == client.lease_id {
                Ok(VaultResponse {
                    request_id: String::new(),
                    lease_id: Some(client.lease_id.clone()),
                    renewable: Some(client.renewable),
                    lease_duration: Some(VaultDuration(client.lease_duration)),
                    data: None,
                    warnings: None,
                    auth: None,
                    wrap_info: None,
                })
            } else {
                Err(Error::VaultNoDataError)
            }
        }

        async fn get_lease(&self, role: &str) -> Result<VaultResponse<PostgresqlLogin>, Error> {
            assert_eq!(role, self.read().await.role);

            {
                self.write().await.counts.write().unwrap().get_count += 1;
            }

            let client = self.read().await;

            Ok(VaultResponse {
                request_id: String::new(),
                lease_id: Some(client.lease_id.clone()),
                renewable: Some(client.renewable),
                lease_duration: Some(VaultDuration(client.lease_duration)),
                data: Some(PostgresqlLogin {
                    username: client.postgres_user.clone(),
                    password: client.postgres_password.clone(),
                }),
                warnings: None,
                auth: None,
                wrap_info: None,
            })
        }
    }

    #[tokio::test(start_paused = true)]
    async fn renews_role_lease() {
        let shutdown = crate::graceful_shutdown::GracefulShutdown::new();
        let vault_client = Arc::new(RwLock::new(MockVaultClient {
            lease_id: "l1".to_string(),
            lease_duration: Duration::from_secs(300),
            renewable: true,
            role: "dbrole".to_string(),
            postgres_user: "username".to_string(),
            postgres_password: "pwd".to_string(),

            counts: std::sync::RwLock::new(MockData::new()),
        }));

        let m = Manager::new(
            VaultPostgresPoolAuth::Vault {
                client: vault_client.clone(),
                role: "dbrole".to_string(),
            },
            shutdown.consumer(),
            "host".to_string(),
            5432,
            "database".to_string(),
        )
        .await
        .unwrap();

        vault_client.read().await.check_counts(1, 0);

        let mut stats_receiver = m.0.stats.clone();
        // Clear the changed flag since it's set by default
        assert!(stats_receiver.changed().await.is_ok());

        tokio::time::sleep(Duration::from_secs(140)).await;
        vault_client.read().await.check_counts(1, 0);
        tokio::time::sleep(Duration::from_secs(20)).await;

        assert!(stats_receiver.changed().await.is_ok());
        let stats = m.0.stats.borrow();
        assert_eq!(
            *stats,
            ManagerStats {
                renew_failures: 0,
                renew_successes: 1,
                update_failures: 0,
                update_successes: 1,
            }
        );
    }

    #[tokio::test(start_paused = true)]
    async fn updates_nonrenewable_lease() {
        let shutdown = crate::graceful_shutdown::GracefulShutdown::new();
        let vault_client = Arc::new(RwLock::new(MockVaultClient {
            lease_id: "l1".to_string(),
            lease_duration: Duration::from_secs(300),
            renewable: false,
            role: "dbrole".to_string(),
            postgres_user: "username".to_string(),
            postgres_password: "pwd".to_string(),

            counts: std::sync::RwLock::new(MockData::new()),
        }));

        let m = Manager::new(
            VaultPostgresPoolAuth::Vault {
                client: vault_client.clone(),
                role: "dbrole".to_string(),
            },
            shutdown.consumer(),
            "host".to_string(),
            5432,
            "database".to_string(),
        )
        .await
        .unwrap();

        vault_client.read().await.check_counts(1, 0);

        let mut stats_receiver = m.0.stats.clone();
        // Clear the changed flag since it's set by default
        assert!(stats_receiver.changed().await.is_ok());

        tokio::time::sleep(Duration::from_secs(140)).await;
        vault_client.read().await.check_counts(1, 0);
        tokio::time::sleep(Duration::from_secs(20)).await;

        assert!(stats_receiver.changed().await.is_ok());
        let stats = m.0.stats.borrow();
        assert_eq!(
            *stats,
            ManagerStats {
                renew_failures: 0,
                renew_successes: 0,
                update_failures: 0,
                update_successes: 2,
            }
        );
    }
}
