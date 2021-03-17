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

pub struct VaultPostgresPool {
    pool: Pool<WrappedConnection, Error>,
}

impl VaultPostgresPool {
    pub fn new(
        max_connections: usize,
        host: String,
        database: String,
        role: String,
        vault_client: Arc<RwLock<VaultClient<()>>>,
    ) -> Result<Arc<VaultPostgresPool>, Error> {
        let manager = Manager::new(vault_client, host, database, role)?;

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
