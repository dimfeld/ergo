use deadpool::managed::Pool;
use derivative::Derivative;
use hashicorp_vault::client::VaultClient;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;
use std::sync::{atomic::Ordering, Arc, RwLock};
use thiserror::Error;

mod conn_executor;
mod connection_manager;
mod vault_token_refresh;
// Disabled until I figure out some lifetime issues
// mod executor;

use connection_manager::{Manager, WrappedConnection};
pub use vault_token_refresh::refresh_vault_client;

pub type SharedVaultClient<T> = Arc<RwLock<VaultClient<T>>>;
pub type TokenAuthVaultClient = SharedVaultClient<hashicorp_vault::client::TokenData>;
pub type AppRoleVaultClient = SharedVaultClient<()>;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    SqlError(#[from] sqlx::error::Error),

    #[error("Vault Error")]
    VaultError(#[from] hashicorp_vault::Error),

    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),

    #[error("Vault returned no auth data")]
    VaultNoDataError,

    #[error("timed out")]
    TimeoutError,
}

impl sqlx::error::DatabaseError for Error {
    fn message(&self) -> &str {
        match self {
            Error::SqlError(sqlx::Error::Database(e)) => e.message(),
            _ => "",
        }
    }

    fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        self
    }

    fn as_error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
        self
    }

    fn into_error(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync + 'static> {
        self
    }
}

pub type ConnectionObject = deadpool::managed::Object<WrappedConnection, Error>;

pub struct VaultPostgresPoolOptions<T: DeserializeOwned + Send + Sync> {
    pub max_connections: usize,
    pub host: String,
    pub database: String,
    pub role: String,
    pub vault_client: SharedVaultClient<T>,
    pub shutdown: graceful_shutdown::GracefulShutdownConsumer,
}

#[derive(Clone, Debug, Serialize)]
pub struct VaultPostgresPoolStats {
    pub renew_successes: u64,
    pub renew_failures: u64,
    pub update_successes: u64,
    pub update_failures: u64,
}

#[derive(Derivative)]
#[derivative(Debug = "transparent")]
pub struct VaultPostgresPool<T: 'static + DeserializeOwned + Send + Sync>(
    pub(crate) Arc<VaultPostgresPoolInner<T>>,
);

#[macro_export]
macro_rules! execute {
    ($pool: expr) => {
        &mut **$pool.acquire().await?
    };
}

fn debug_format_pool(
    p: &Pool<WrappedConnection, Error>,
    fmt: &mut std::fmt::Formatter,
) -> Result<(), std::fmt::Error> {
    p.status().fmt(fmt)
}

#[derive(Derivative)]
#[derivative(Debug)]
struct VaultPostgresPoolInner<T: 'static + DeserializeOwned + Send + Sync> {
    manager: Arc<Manager<VaultClient<T>>>,
    #[derivative(Debug(format_with = "debug_format_pool"))]
    pool: Pool<WrappedConnection, Error>,
}

fn unwrap_pool_error(e: deadpool::managed::PoolError<Error>) -> Error {
    match e {
        deadpool::managed::PoolError::Timeout(_) => Error::TimeoutError,
        deadpool::managed::PoolError::Backend(e) => e,
    }
}

impl<T: 'static + DeserializeOwned + Send + Sync> VaultPostgresPool<T> {
    pub fn new(config: VaultPostgresPoolOptions<T>) -> Result<VaultPostgresPool<T>, Error> {
        let VaultPostgresPoolOptions {
            max_connections,
            host,
            database,
            role,
            vault_client,
            shutdown,
        } = config;
        let manager = Manager::new(vault_client, shutdown, host, database, role)?;

        let pool = VaultPostgresPoolInner {
            manager: manager.clone(),
            pool: Pool::new(manager, max_connections),
        };

        Ok(VaultPostgresPool(Arc::new(pool)))
    }

    pub fn stats(&self) -> VaultPostgresPoolStats {
        VaultPostgresPoolStats {
            renew_successes: self.0.manager.renew_successes.load(Ordering::Relaxed),
            renew_failures: self.0.manager.renew_failures.load(Ordering::Relaxed),
            update_successes: self.0.manager.update_successes.load(Ordering::Relaxed),
            update_failures: self.0.manager.update_failures.load(Ordering::Relaxed),
        }
    }

    pub async fn acquire(&self) -> Result<ConnectionObject, Error> {
        self.0.pool.get().await.map_err(unwrap_pool_error)
    }

    pub async fn try_acquire(&self) -> Result<ConnectionObject, Error> {
        self.0.pool.try_get().await.map_err(unwrap_pool_error)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
