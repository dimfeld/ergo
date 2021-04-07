use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{App, HttpServer};
use graceful_shutdown::GracefulShutdown;
use hashicorp_vault::client::VaultClient;
use std::{
    env,
    sync::{Arc, RwLock},
};
use tracing_actix_web::TracingLogger;

mod auth;
mod database;
mod error;
mod graceful_shutdown;
mod queues;
mod service_config;
mod tasks;
mod tracing_config;
mod vault;
mod web_app_server;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    dotenv::from_filename("vault_dev_roles.env").ok();

    tracing_config::configure("ergo-server");

    let shutdown = GracefulShutdown::new();

    let vault_address =
        env::var("VAULT_ADDR").unwrap_or_else(|_| "http://localhost:8200".to_string());
    let vault_role_id = env::var("VAULT_ROLE_ERGO_AIO_SERVER_ID")
        .expect("VAULT_ROLE_ERGO_AIO_SERVER_ID is required");
    let vault_secret_id = env::var("VAULT_ROLE_ERGO_AIO_SERVER_SECRET")
        .expect("VAULT_ROLE_ERGO_AIO_SERVER_SECRET is required");
    let vault_client =
        VaultClient::new_app_role(vault_address, vault_role_id, Some(vault_secret_id))
            .expect("Creating vault client");

    let vault_client = Arc::new(RwLock::new(vault_client));
    tracing::info!("{:?}", vault_client);
    vault::refresh_vault_client(vault_client.clone(), shutdown.consumer());

    let address = env::var("BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("BIND_PORT")
        .map(|s| s.parse::<u16>())
        .unwrap_or(Ok(6543))
        .expect("PORT");
    let web_config = service_config::Config {
        vault_client: vault_client.clone(),
        database: env::var("DATABASE").ok(),
        database_host: env::var("DATABASE_HOST").unwrap_or_else(|_| "localhost:5432".to_string()),
        database_role: env::var("DATABASE_ROLE_WEB").ok(),
        shutdown: shutdown.consumer(),
    };

    let redis_host = env::var("REDIS_URL").expect("REDIS_URL is required");
    let redis_pool = deadpool_redis::Config {
        url: Some(redis_host),
        pool: None,
    }
    .create_pool()
    .expect("Creating redis pool");

    let web_app_data = web_app_server::app_data(web_config)?;

    let cookie_signing_key = env::var("COOKIE_SIGNING_KEY")
        .ok()
        .unwrap_or_else(|| {
            println!(
                r##"WARNING: Using default cookie signing key.
            Set COOKIE_SIGNING_KEY environment variable to a 32-byte string to set it"##
            );

            "wpvuwm4pvoane;bwn40s;wmvlscvG@sV".to_string()
        })
        .into_bytes();

    HttpServer::new(move || {
        let identity = IdentityService::new(
            CookieIdentityPolicy::new(&cookie_signing_key)
                .http_only(true)
                .secure(true)
                .same_site(cookie::SameSite::Strict),
        );

        App::new()
            .wrap(TracingLogger)
            .wrap(identity)
            .service(web_app_server::scope(&web_app_data, "/api/web"))
    })
    .bind(format!("{}:{}", address, port))?
    .run()
    .await?;

    shutdown.shutdown().await?;
    Ok(())
}
