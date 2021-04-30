#![allow(dead_code, unused_imports, unused_variables)] // Remove this once the basic application is up and working

//! A server that includes all the functionality in one executable.
//! Mostly useful for development or test purposes.

use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{App, HttpServer};
use hashicorp_vault::client::VaultClient;
use std::{
    env,
    sync::{Arc, RwLock},
};
use tracing::{event, Level};
use tracing_actix_web::TracingLogger;

use ergo::{
    database::VaultPostgresPoolAuth,
    graceful_shutdown::GracefulShutdown,
    tasks::{
        self, actions::queue::ActionQueue, inputs::queue::InputQueue, runtime::TaskExecutorConfig,
    },
    web_app_server,
};

#[actix_web::main]
async fn main() -> Result<(), ergo::error::Error> {
    dotenv::dotenv().ok();
    dotenv::from_filename("vault_dev_roles.env").ok();

    ergo::tracing_config::configure("ergo-server");

    let shutdown = GracefulShutdown::new();

    let vault_client = ergo::vault::from_env("AIO_SERVER", &shutdown);
    let auth = tracing::info!("{:?}", vault_client);

    let address = env::var("BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("BIND_PORT")
        .map(|s| s.parse::<u16>())
        .unwrap_or(Ok(6543))
        .expect("PORT");
    let web_config = ergo::service_config::Config::new(
        VaultPostgresPoolAuth::from_env(&vault_client, "WEB", "ergo_web")?,
        &shutdown,
    )?;

    let backend_config = ergo::service_config::Config::new(
        VaultPostgresPoolAuth::from_env(&vault_client, "BACKEND", "ergo_backend")?,
        &shutdown,
    )?;

    let redis_host = env::var("REDIS_URL").expect("REDIS_URL is required");
    let redis_pool = deadpool_redis::Config {
        url: Some(redis_host),
        pool: None,
    }
    .create_pool()
    .expect("Creating redis pool");

    let input_queue = InputQueue::new(redis_pool.clone());
    let action_queue = ActionQueue::new(redis_pool.clone());

    let web_app_data = ergo::web_app_server::app_data(web_config.pg_pool);
    let backend_app_data = ergo::tasks::handlers::app_data(
        backend_config.pg_pool.clone(),
        input_queue.clone(),
        action_queue.clone(),
    )?;

    let queue_drain = ergo::tasks::queue_drain_runner::AllQueuesDrain::new(
        input_queue.clone(),
        action_queue.clone(),
        backend_app_data.get_ref().pg.clone(),
        shutdown.consumer(),
    );

    let input_runner = ergo::tasks::runtime::TaskExecutor::new(TaskExecutorConfig {
        redis_pool: redis_pool.clone(),
        pg_pool: backend_app_data.get_ref().pg.clone(),
        shutdown: shutdown.consumer(),
        max_concurrent_jobs: None,
    });

    let cookie_signing_key = env::var("COOKIE_SIGNING_KEY")
        .ok()
        .unwrap_or_else(|| {
            event!(
                Level::WARN,
                "Using default cookie signing key. Set COOKIE_SIGNING_KEY environment variable to a 32-byte string to set it"
            );

            "wpvuwm4pvoane;bwn40s;wmvlscvG@sV".to_string()
        })
        .into_bytes();

    HttpServer::new(move || {
        let identity = IdentityService::new(
            CookieIdentityPolicy::new(&cookie_signing_key)
                .http_only(true)
                .secure(true)
                .same_site(actix_web::cookie::SameSite::Strict),
        );

        App::new()
            .wrap(TracingLogger)
            .wrap(identity)
            .service(web_app_server::scope(&web_app_data, "/api/web"))
            .service(tasks::handlers::scope(&backend_app_data, "/api/tasks"))
    })
    .bind(format!("{}:{}", address, port))?
    .run()
    .await?;

    shutdown.shutdown().await?;
    Ok(())
}
