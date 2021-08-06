use crate::{
    database::{
        PostgresAuthRenewer, VaultPostgresPool, VaultPostgresPoolAuth, VaultPostgresPoolOptions,
    },
    error::Error,
    graceful_shutdown::GracefulShutdownConsumer,
};
use std::{env, sync::Arc};

#[derive(Clone, Debug)]
pub struct DatabaseConfiguration {
    pub host: String,
    pub port: u16,
    pub database: String,
}

impl Default for DatabaseConfiguration {
    fn default() -> Self {
        database_configuration_from_env().unwrap()
    }
}

pub fn database_configuration_from_env() -> Result<DatabaseConfiguration, Error> {
    Ok(DatabaseConfiguration {
        host: env::var("DATABASE_HOST").unwrap_or_else(|_| "localhost".to_string()),
        port: envoption::with_default("DATABASE_PORT", 5432 as u16)?,
        database: env::var("DATABASE").unwrap_or_else(|_| "ergo".to_string()),
    })
}

async fn pg_pool(
    shutdown: GracefulShutdownConsumer,
    auth: VaultPostgresPoolAuth,
    configuration: DatabaseConfiguration,
) -> Result<VaultPostgresPool, Error> {
    VaultPostgresPool::new(VaultPostgresPoolOptions {
        max_connections: 16,
        host: configuration.host,
        port: configuration.port,
        database: configuration.database,
        auth,
        shutdown,
    })
    .await
}

pub async fn backend_pg_pool(
    shutdown: GracefulShutdownConsumer,
    vault_client: &Option<Arc<dyn PostgresAuthRenewer>>,
    configuration: DatabaseConfiguration,
) -> Result<VaultPostgresPool, Error> {
    pg_pool(
        shutdown,
        VaultPostgresPoolAuth::from_env(vault_client, "BACKEND", "ergo_backend")?,
        configuration,
    )
    .await
}

pub async fn web_pg_pool(
    shutdown: GracefulShutdownConsumer,
    vault_client: &Option<Arc<dyn PostgresAuthRenewer>>,
    configuration: DatabaseConfiguration,
) -> Result<VaultPostgresPool, Error> {
    pg_pool(
        shutdown,
        VaultPostgresPoolAuth::from_env(vault_client, "WEB", "ergo_web")?,
        configuration,
    )
    .await
}