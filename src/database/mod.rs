use crate::{
    error::Error,
    vault::{SharedVaultClient, VaultClientTokenData},
};
use deadpool::managed::Pool;
use derivative::Derivative;
use hashicorp_vault::client::VaultClient;
use itertools::Itertools;
use serde::de::DeserializeOwned;
use std::{
    env,
    fmt::Debug,
    sync::{Arc, RwLock},
};

mod conn_executor;
mod connection_manager;
mod executor;
pub mod transaction;

use connection_manager::{Manager, WrappedConnection};

pub use self::connection_manager::PostgresAuthRenewer;

pub type ConnectionObject = deadpool::managed::Object<WrappedConnection, Error>;
pub type PostgresPool = VaultPostgresPool;

#[derive(Clone, Debug)]
pub enum VaultPostgresPoolAuth {
    Vault {
        client: Arc<dyn PostgresAuthRenewer>,
        role: String,
    },
    Password {
        username: String,
        password: String,
    },
}

impl VaultPostgresPoolAuth {
    pub fn from_env(
        vault_client: &Option<Arc<dyn PostgresAuthRenewer>>,
        database_role_env_name: &str,
        default_vault_role: &str,
    ) -> Result<Self, Error> {
        if let Some(client) = vault_client {
            let db_role_env = env::var(&format!("DATABASE_ROLE_{}", database_role_env_name))
                .unwrap_or_else(|_| default_vault_role.to_string());
            return Ok(Self::Vault {
                client: client.clone(),
                role: db_role_env,
            });
        }

        let db_username_env = format!("DATABASE_ROLE_{}_USERNAME", database_role_env_name);
        let db_password_env = format!("DATABASE_ROLE_{}_PASSWORD", database_role_env_name);
        let db_username =
            env::var(&db_username_env).unwrap_or_else(|_| default_vault_role.to_string());
        let db_password = env::var(&db_password_env);

        match db_password {
            Ok(password) => Ok(Self::Password{username: db_username, password}),
            _ =>
                Err(Error::ConfigError(format!("Environment must have Vault AppRole configuration (VAULT_* env) or Fixed database credentials ({}, {}) settings, but not both",
                    db_username_env, db_password_env
                ))),
        }
    }
}

pub struct VaultPostgresPoolOptions {
    pub max_connections: usize,
    pub host: String,
    pub database: String,
    pub auth: VaultPostgresPoolAuth,
    pub shutdown: crate::graceful_shutdown::GracefulShutdownConsumer,
}

#[derive(Derivative)]
#[derivative(Debug = "transparent")]
pub struct VaultPostgresPool(Arc<VaultPostgresPoolInner>);

impl Clone for VaultPostgresPool {
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
struct VaultPostgresPoolInner {
    manager: Arc<Manager>,
    #[derivative(Debug(format_with = "debug_format_pool"))]
    pool: Pool<WrappedConnection, Error>,
}

fn unwrap_pool_error(e: deadpool::managed::PoolError<Error>) -> Error {
    match e {
        deadpool::managed::PoolError::Timeout(_) => Error::TimeoutError,
        deadpool::managed::PoolError::Backend(e) => e,
    }
}

impl VaultPostgresPool {
    pub fn new(config: VaultPostgresPoolOptions) -> Result<VaultPostgresPool, Error> {
        let VaultPostgresPoolOptions {
            max_connections,
            host,
            database,
            auth,
            shutdown,
        } = config;
        let manager = Manager::new(auth, shutdown, host, database)?;

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

pub fn sql_insert_parameters<const NCOL: usize>(num_rows: usize) -> String {
    (0..num_rows)
        .into_iter()
        .map(|i| {
            let base = i * NCOL + 1;
            let mut output = String::with_capacity(2 + NCOL * 4);

            output.push('(');
            output.push('$');
            output.push_str(base.to_string().as_str());
            for i in 1..NCOL {
                output.push_str(",$");
                output.push_str((base + i).to_string().as_str());
            }
            output.push(')');

            output
        })
        .join(",\n")
}

#[cfg(test)]
mod tests {
    use super::sql_insert_parameters as sip;

    #[test]
    fn sql_insert_parameters() {
        assert_eq!(
            sip::<2>(3),
            r##"($1,$2),
($3,$4),
($5,$6)"##
        );
    }
}
