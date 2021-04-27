use crate::{
    database::PostgresAuthRenewer,
    graceful_shutdown::{GracefulShutdown, GracefulShutdownConsumer},
};
use hashicorp_vault::client::{TokenData, VaultClient};
use serde::de::DeserializeOwned;
use std::{
    env,
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::{select, task::JoinHandle};
use tracing::{event, Level};

pub type SharedVaultClient<T> = Arc<RwLock<VaultClient<T>>>;
pub type TokenAuthVaultClient = SharedVaultClient<hashicorp_vault::client::TokenData>;
pub type AppRoleVaultClient = SharedVaultClient<()>;

pub trait VaultClientTokenData: std::fmt::Debug + DeserializeOwned + Send + Sync + 'static {}
impl VaultClientTokenData for TokenData {}
impl VaultClientTokenData for () {}

async fn vault_client_renew_loop<T: 'static + DeserializeOwned + Send + Sync>(
    client: SharedVaultClient<T>,
    mut shutdown: GracefulShutdownConsumer,
) {
    let lease_renew_duration = client
        .read()
        .unwrap()
        .data
        .as_ref()
        .and_then(|d| d.auth.as_ref())
        .and_then(|a| a.lease_duration.as_ref())
        .map(|d| d.0)
        .unwrap_or(Duration::from_secs(600))
        .div_f32(2.0);

    let mut next_wait = tokio::time::Instant::now() + lease_renew_duration;

    event!(
        Level::INFO,
        "Renewing Vault auth every {:?}",
        lease_renew_duration
    );

    loop {
        select! {
            _ = tokio::time::sleep_until(next_wait) => {
                let c = client.clone();
                // TODO Error handling, retry, etc.
                event!(Level::INFO, "Refreshing vault client auth");
                let result = tokio::task::spawn_blocking(move || c.write().unwrap().renew()).await.unwrap();
                match result {
                    Ok(_) => event!(Level::INFO, "Done refreshing vault client auth"),
                    Err(e) => event!(Level::ERROR, error=?e, "Error refreshing vault client auth"),
                };

                next_wait = tokio::time::Instant::now() + lease_renew_duration;
            },
            _ = shutdown.wait_for_shutdown() => {
                break;
            }
        }
    }
}

fn refresh_vault_client<T: 'static + DeserializeOwned + Send + Sync>(
    client: SharedVaultClient<T>,
    shutdown: GracefulShutdownConsumer,
) -> JoinHandle<()> {
    tokio::spawn(vault_client_renew_loop(client, shutdown))
}

pub fn from_env(
    env_name: &str,
    shutdown: &GracefulShutdown,
) -> Option<Arc<dyn PostgresAuthRenewer>> {
    let vault_address =
        env::var("VAULT_ADDR").unwrap_or_else(|_| "http://localhost:8200".to_string());
    let vault_role_id_env = format!("VAULT_ROLE_ERGO_{}_ID", env_name);
    let vault_secret_id_env = format!("VAULT_ROLE_ERGO_{}_SECRET", env_name);

    let vault_role_id = env::var(&vault_role_id_env);
    let vault_secret_id = env::var(&vault_secret_id_env);

    match (vault_role_id, vault_secret_id) {
        (Ok(vault_role_id), Ok(vault_secret_id)) => {
            let client =
                VaultClient::new_app_role(vault_address, vault_role_id, Some(vault_secret_id))
                    .expect("Creating vault client");
            let client = Arc::new(RwLock::new(client));
            refresh_vault_client(client.clone(), shutdown.consumer());
            Some(client as Arc<dyn PostgresAuthRenewer>)
        }
        _ => None,
    }
}
