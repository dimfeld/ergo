use crate::{
    database::VaultPostgresPoolAuth,
    graceful_shutdown::{self, GracefulShutdown},
    vault::VaultClientTokenData,
};
use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub database: Option<String>,
    pub database_host: String,
    pub database_auth: VaultPostgresPoolAuth,

    pub redis_host: String,

    pub shutdown: graceful_shutdown::GracefulShutdownConsumer,
}

impl Config {
    pub fn new(auth: VaultPostgresPoolAuth, shutdown: &GracefulShutdown) -> Self {
        Config {
            database: env::var("DATABASE").ok(),
            database_auth: auth,
            database_host: env::var("DATABASE_HOST")
                .unwrap_or_else(|_| "localhost:5432".to_string()),
            redis_host: env::var("REDIS_HOST").expect("REDIS_HOST is required"),
            shutdown: shutdown.consumer(),
        }
    }
}
