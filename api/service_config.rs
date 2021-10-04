use crate::error::Error;
use ergo_database::{PostgresAuth, PostgresPool};
use log::LevelFilter;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    ConnectOptions,
};
use std::env;

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
    auth: PostgresAuth,
    configuration: &DatabaseConfiguration,
) -> Result<PostgresPool, Error> {
    let mut connect_options = PgConnectOptions::new()
        .host(&configuration.host)
        .port(configuration.port)
        .username(&auth.username)
        .password(&auth.password)
        .database(&configuration.database);

    connect_options.log_statements(LevelFilter::Debug);

    PgPoolOptions::new()
        .max_connections(16)
        .max_lifetime(Some(std::time::Duration::from_secs(3600 * 12)))
        .connect_timeout(std::time::Duration::from_secs(30))
        .connect_with(connect_options)
        .await
        .map_err(|e| e.into())
}

pub async fn backend_pg_pool(configuration: &DatabaseConfiguration) -> Result<PostgresPool, Error> {
    pg_pool(
        PostgresAuth::from_env("BACKEND", "ergo_backend")?,
        configuration,
    )
    .await
}

pub async fn web_pg_pool(configuration: &DatabaseConfiguration) -> Result<PostgresPool, Error> {
    pg_pool(PostgresAuth::from_env("WEB", "ergo_web")?, configuration).await
}
