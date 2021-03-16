use hashicorp_vault::client::VaultClient;
use std::env;
use std::sync::{Arc, RwLock};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    dotenv::from_filename("vault_dev_roles.env").ok();

    let vault_address =
        env::var("VAULT_ADDRESS").unwrap_or_else(|_| "http://localhost:8200".to_string());
    let vault_role_id = env::var("VAULT_ROLE_ERGO_AIO_SERVER_ID")
        .expect("VAULT_ROLE_ERGO_AIO_SERVER_ID is required");
    let vault_secret_id = env::var("VAULT_ROLE_ERGO_AIO_SERVER_SECRET")
        .expect("VAULT_ROLE_ERGO_AIO_SERVER_SECRET is required");
    let vault_client = VaultClient::new_app_role(vault_address, vault_role_id, vault_secret_id)
        .expect("Creating vault client");

    let vault_client = Arc::new(RwLock::new(vault_client));

    let web_config = web_server::Config {
        address: env::var("BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1".to_string()),
        port: env::var("BIND_PORT")
            .map(|s| s.parse::<u16>())
            .unwrap_or(Ok(6543))
            .expect("PORT"),
        vault_client: vault_client.clone(),
    };

    web_server::new(&web_config)?.await
}
