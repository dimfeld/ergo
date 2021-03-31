use crate::{error::Error, vault::SharedVaultClient};
use deadpool::managed::Pool;
use derivative::Derivative;
use hashicorp_vault::client::VaultClient;
use serde::de::DeserializeOwned;
use std::{fmt::Debug, sync::Arc};

mod conn_executor;
mod connection_manager;
mod executor;
pub mod transaction;

use connection_manager::{Manager, WrappedConnection};

pub type ConnectionObject = deadpool::managed::Object<WrappedConnection, Error>;

pub struct VaultPostgresPoolOptions<T: DeserializeOwned + Send + Sync> {
    pub max_connections: usize,
    pub host: String,
    pub database: String,
    pub role: String,
    pub vault_client: SharedVaultClient<T>,
    pub shutdown: crate::graceful_shutdown::GracefulShutdownConsumer,
}

#[derive(Derivative)]
#[derivative(Debug = "transparent")]
pub struct VaultPostgresPool<T: 'static + DeserializeOwned + Send + Sync>(
    Arc<VaultPostgresPoolInner<T>>,
);

impl<T: 'static + DeserializeOwned + Send + Sync> Clone for VaultPostgresPool<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
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

    pub fn stats(&self) -> connection_manager::ManagerStats {
        self.0.manager.stats.borrow().clone()
    }

    pub fn stats_receiver(&self) -> tokio::sync::watch::Receiver<connection_manager::ManagerStats> {
        self.0.manager.stats.clone()
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
