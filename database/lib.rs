use deadpool::managed::Pool;
use itertools::Itertools;
use std::{env, fmt::Debug, sync::Arc};
use thiserror::Error;

mod conn_executor;
mod connection_manager;
mod executor;
pub mod object_id;
pub mod redis;
pub mod transaction;

pub use self::{object_id::new_object_id, redis::RedisPool};

use connection_manager::{Manager, WrappedConnection};

pub use self::connection_manager::PostgresAuthRenewer;

pub type ConnectionObject = deadpool::managed::Object<Manager>;
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
    pub port: u16,
    pub database: String,
    pub auth: VaultPostgresPoolAuth,
    pub shutdown: ergo_graceful_shutdown::GracefulShutdownConsumer,
}

#[derive(Clone)]
pub struct VaultPostgresPool(Pool<Manager>);

impl std::fmt::Debug for VaultPostgresPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresPool")
            .field("pool", &self.0.status())
            .field("manager", &self.0.manager())
            .finish()
    }
}

fn unwrap_pool_error(e: deadpool::managed::PoolError<Error>) -> Error {
    match e {
        deadpool::managed::PoolError::Timeout(_) => Error::TimeoutError,
        deadpool::managed::PoolError::Backend(e) => e,
        deadpool::managed::PoolError::Closed => Error::PoolClosed,
        deadpool::managed::PoolError::NoRuntimeSpecified => {
            Error::StringError("no runtime specified".to_string())
        }
    }
}

impl VaultPostgresPool {
    pub async fn new(config: VaultPostgresPoolOptions) -> Result<VaultPostgresPool, Error> {
        let VaultPostgresPoolOptions {
            max_connections,
            host,
            port,
            database,
            auth,
            shutdown,
        } = config;
        let manager = Manager::new(auth, shutdown, host, port, database).await?;

        let pool = Pool::new(manager, max_connections);

        Ok(VaultPostgresPool(pool))
    }

    pub fn stats(&self) -> connection_manager::ManagerStats {
        self.0.manager().0.stats.borrow().clone()
    }

    pub fn stats_receiver(&self) -> tokio::sync::watch::Receiver<connection_manager::ManagerStats> {
        self.0.manager().0.stats.clone()
    }

    pub async fn acquire(&self) -> Result<ConnectionObject, Error> {
        self.0.get().await.map_err(unwrap_pool_error)
    }

    pub async fn try_acquire(&self) -> Result<ConnectionObject, Error> {
        self.0.try_get().await.map_err(unwrap_pool_error)
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

#[derive(Debug, Error)]
pub enum Error {
    #[error("Unable to execute serializable transaction")]
    SerializationFailure,

    #[error("Tried to create object ID {id} with type {wanted} but it has type {saw}")]
    ObjectIdTypeMismatch {
        id: i64,
        wanted: String,
        saw: String,
    },

    #[error("{0}")]
    StringError(String),

    #[error("SQL Error")]
    SqlError(#[from] sqlx::error::Error),

    #[error("Database Configuration Error: {0}")]
    ConfigError(String),

    #[error("Connection pool closed")]
    PoolClosed,

    #[error("timed out")]
    TimeoutError,

    #[error("Redis connection error {0}")]
    RedisPoolError(#[from] deadpool::managed::PoolError<::redis::RedisError>),

    #[error("Redis pool creation error {0}")]
    RedisPoolCreationError(#[from] deadpool_redis::CreatePoolError),

    #[error("Vault Error")]
    VaultError(#[from] hashicorp_vault::Error),

    #[error("Vault returned no auth data")]
    VaultNoDataError,
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
