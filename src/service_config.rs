use crate::{
    database::{VaultPostgresPool, VaultPostgresPoolAuth, VaultPostgresPoolOptions},
    error::Error,
    graceful_shutdown::{self, GracefulShutdown},
    vault::VaultClientTokenData,
};
use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub pg_pool: VaultPostgresPool,
    pub redis_host: String,
    pub shutdown: graceful_shutdown::GracefulShutdownConsumer,
}

impl Config {
    pub fn new(auth: VaultPostgresPoolAuth, shutdown: &GracefulShutdown) -> Result<Self, Error> {
        let database = env::var("DATABASE").unwrap_or_else(|_| "ergo".to_string());
        let database_host =
            env::var("DATABASE_HOST").unwrap_or_else(|_| "localhost:5432".to_string());

        let pg_pool = VaultPostgresPool::new(VaultPostgresPoolOptions {
            max_connections: 16,
            host: database_host,
            database,
            auth,
            shutdown: shutdown.consumer(),
        })?;

        Ok(Config {
            pg_pool,
            redis_host: env::var("REDIS_HOST").expect("REDIS_HOST is required"),
            shutdown: shutdown.consumer(),
        })
    }
}
