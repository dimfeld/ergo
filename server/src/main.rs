use graceful_shutdown::GracefulShutdown;
use hashicorp_vault::client::VaultClient;
use std::env;
use std::sync::{Arc, RwLock};

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

    let web_config = web_server::Config {
        address: env::var("BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1".to_string()),
        port: env::var("BIND_PORT")
            .map(|s| s.parse::<u16>())
            .unwrap_or(Ok(6543))
            .expect("PORT"),
        vault_client: vault_client.clone(),
        database: env::var("DATABASE").ok(),
        database_host: env::var("DATABASE_HOST").unwrap_or_else(|_| "localhost:5432".to_string()),
        database_role: env::var("DATABASE_ROLE_WEB").ok(),
        shutdown: shutdown.consumer(),
    };

    web_server::new(web_config)?.await
}
