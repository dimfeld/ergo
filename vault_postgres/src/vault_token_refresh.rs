use hashicorp_vault::client::VaultClient;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::select;
use tokio::signal::ctrl_c;
use tokio::task::JoinHandle;
use tracing::{event, Level};

async fn vault_client_renew_loop(client: Arc<RwLock<VaultClient<()>>>) {
    let lease_renew_wait = client
        .read()
        .unwrap()
        .data
        .as_ref()
        .and_then(|d| d.auth.as_ref())
        .and_then(|a| a.lease_duration.as_ref())
        .map(|d| d.0)
        .unwrap_or(Duration::from_secs(600))
        .div_f32(2.0);

    event!(
        Level::INFO,
        "Renewing Vault auth every {:?}",
        lease_renew_wait
    );

    loop {
        select! {
            _ = tokio::time::sleep(lease_renew_wait) => {
                let c = client.clone();
                // TODO Error handling, retry, etc.
                event!(Level::INFO, "Refreshing vault client auth");
                let result = tokio::task::spawn_blocking(move || c.write().unwrap().renew()).await.unwrap();
                match result {
                    Ok(_) => event!(Level::INFO, "Done refreshing vault client auth"),
                    Err(e) => event!(Level::ERROR, error=?e, "Error refreshing vault client auth"),
                };
            },
            _ = ctrl_c() => {
                break;
            }
        }
    }
}

pub fn refresh_vault_client(client: Arc<RwLock<VaultClient<()>>>) -> JoinHandle<()> {
    tokio::spawn(vault_client_renew_loop(client))
}
