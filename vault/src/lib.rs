use deadpool::managed::Pool;
use hashicorp_vault::client::VaultClient;
use std::sync::{Arc, RwLock};
use thiserror::Error;

mod connection_manager;
mod vault_token_refresh;

use connection_manager::{Manager, WrappedConnection};
pub use vault_token_refresh::refresh_vault_client;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    SqlError(#[from] sqlx::error::Error),

    #[error("Vault Error")]
    VaultError(#[from] hashicorp_vault::Error),

    #[error("Vault returned no auth data")]
    VaultNoDataError,
}

pub type ConnectionObject = deadpool::managed::Object<WrappedConnection, Error>;

pub struct VaultPostgresPoolOptions {
    pub max_connections: usize,
    pub host: String,
    pub database: String,
    pub role: String,
    pub vault_client: Arc<RwLock<VaultClient<()>>>,
    pub shutdown: graceful_shutdown::GracefulShutdownConsumer,
}

pub struct VaultPostgresPool {
    pool: Pool<WrappedConnection, Error>,
}

impl VaultPostgresPool {
    pub fn new(config: VaultPostgresPoolOptions) -> Result<Arc<VaultPostgresPool>, Error> {
        let VaultPostgresPoolOptions {
            max_connections,
            host,
            database,
            role,
            vault_client,
            shutdown,
        } = config;
        let manager = Manager::new(vault_client, shutdown, host, database, role)?;

        let pool = VaultPostgresPool {
            pool: Pool::new(manager, max_connections),
        };

        Ok(Arc::new(pool))
    }

    pub async fn acquire(&self) -> Result<ConnectionObject, deadpool::managed::PoolError<Error>> {
        self.pool.get().await
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
