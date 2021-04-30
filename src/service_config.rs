use crate::{
    database::{
        PostgresAuthRenewer, VaultPostgresPool, VaultPostgresPoolAuth, VaultPostgresPoolOptions,
    },
    error::Error,
    graceful_shutdown::{self, GracefulShutdown, GracefulShutdownConsumer},
    vault::VaultClientTokenData,
};
use std::{env, sync::Arc};

async fn pg_pool(
    shutdown: GracefulShutdownConsumer,
    auth: VaultPostgresPoolAuth,
) -> Result<VaultPostgresPool, Error> {
    let database = env::var("DATABASE").unwrap_or_else(|_| "ergo".to_string());
    let database_host = env::var("DATABASE_HOST").unwrap_or_else(|_| "localhost:5432".to_string());

    VaultPostgresPool::new(VaultPostgresPoolOptions {
        max_connections: 16,
        host: database_host,
        database,
        auth,
        shutdown,
    })
    .await
}

pub fn redis_pool() -> Result<deadpool_redis::Pool, Error> {
    let redis_host = env::var("REDIS_URL").expect("REDIS_URL is required");
    deadpool_redis::Config {
        url: Some(redis_host),
        pool: None,
    }
    .create_pool()
    .map_err(|e| e.into())
}

pub async fn backend_pg_pool(
    shutdown: GracefulShutdownConsumer,
    vault_client: &Option<Arc<dyn PostgresAuthRenewer>>,
) -> Result<VaultPostgresPool, Error> {
    pg_pool(
        shutdown,
        VaultPostgresPoolAuth::from_env(vault_client, "BACKEND", "ergo_backend")?,
    )
    .await
}

pub async fn web_pg_pool(
    shutdown: GracefulShutdownConsumer,
    vault_client: &Option<Arc<dyn PostgresAuthRenewer>>,
) -> Result<VaultPostgresPool, Error> {
    pg_pool(
        shutdown,
        VaultPostgresPoolAuth::from_env(vault_client, "WEB", "ergo_web")?,
    )
    .await
}
