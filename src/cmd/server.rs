use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{
    web::{self, PathConfig},
    App, HttpServer,
};
use hashicorp_vault::client::VaultClient;
use std::{env, sync::Arc};
use structopt::StructOpt;
use tracing::{event, Level};
use tracing_actix_web::TracingLogger;

use crate::{
    auth::{
        middleware::{AuthenticateMiddleware, AuthenticateMiddlewareFactory},
        AuthData,
    },
    database::VaultPostgresPoolAuth,
    graceful_shutdown::GracefulShutdown,
    status_server,
    tasks::{
        self,
        actions::{
            dequeue::{ActionExecutor, ActionExecutorConfig},
            queue::ActionQueue,
        },
        inputs::{
            dequeue::{TaskExecutor, TaskExecutorConfig},
            queue::InputQueue,
        },
    },
    web_app_server,
};

#[derive(Debug, StructOpt)]
pub struct Args {
    #[structopt(long, help = "Do not run the PostgreSQL queue stage drain tasks")]
    no_drain_queues: bool,
}

pub async fn main(args: Args) -> Result<(), crate::error::Error> {
    crate::tracing_config::configure("ergo");
    let shutdown = GracefulShutdown::new();

    let vault_client = crate::vault::from_env("AIO_SERVER", &shutdown).await;
    tracing::info!(
        "Vault mode {}",
        vault_client
            .as_ref()
            .map(|_| "enabled")
            .unwrap_or("disabled")
    );

    let address: String = envoption::with_default("BIND_ADDRESS", "127.0.0.1")?;
    let port: u16 = envoption::with_default("BIND_PORT", 6543 as u16)?;

    let web_pg_pool =
        crate::service_config::web_pg_pool(shutdown.consumer(), &vault_client).await?;
    let backend_pg_pool =
        crate::service_config::backend_pg_pool(shutdown.consumer(), &vault_client).await?;

    let redis_pool = crate::service_config::redis_pool()?;

    let input_queue = InputQueue::new(redis_pool.clone());
    let action_queue = ActionQueue::new(redis_pool.clone());

    let notifications = crate::notifications::NotificationManager::new(
        backend_pg_pool.clone(),
        redis_pool.clone(),
        shutdown.consumer(),
    )?;

    let immediate_actions = envoption::with_default("IMMEDIATE_ACTIONS", false)?;
    let immediate_inputs = envoption::with_default("IMMEDIATE_INPUTS", false)?;

    let web_app_data = crate::web_app_server::app_data(web_pg_pool.clone());
    let backend_app_data = crate::backend_data::app_data(
        backend_pg_pool.clone(),
        notifications.clone(),
        input_queue.clone(),
        action_queue.clone(),
        immediate_inputs,
        immediate_actions,
    )?;

    let queue_drain = if args.no_drain_queues {
        None
    } else {
        Some(crate::tasks::queue_drain_runner::AllQueuesDrain::new(
            input_queue.clone(),
            action_queue.clone(),
            backend_pg_pool.clone(),
            redis_pool.clone(),
            shutdown.consumer(),
        )?)
    };

    let input_runner = TaskExecutor::new(TaskExecutorConfig {
        redis_pool: redis_pool.clone(),
        pg_pool: backend_pg_pool.clone(),
        shutdown: shutdown.consumer(),
        notifications: Some(notifications.clone()),
        max_concurrent_jobs: None,
        immediate_actions,
    })?;

    let action_runner = ActionExecutor::new(ActionExecutorConfig {
        redis_pool: redis_pool.clone(),
        pg_pool: backend_pg_pool.clone(),
        shutdown: shutdown.consumer(),
        notifications: Some(notifications.clone()),
        max_concurrent_jobs: None,
    })?;

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

        App::new().service(
            web::scope("/api")
                .app_data(PathConfig::default().error_handler(|err, req| {
                    event!(Level::ERROR, ?err, ?req);
                    eprintln!("{}", err);
                    actix_web::error::ErrorNotFound(err)
                }))
                .app_data(web_app_data.clone())
                .app_data(backend_app_data.clone())
                .wrap(AuthenticateMiddlewareFactory::new(
                    backend_app_data.auth.clone(),
                ))
                .wrap(identity)
                .wrap(TracingLogger::default())
                .configure(web_app_server::config)
                .configure(tasks::handlers::config)
                .configure(status_server::config),
        )
    })
    .bind(format!("{}:{}", address, port))?
    .run()
    .await?;

    shutdown.shutdown().await?;
    Ok(())
}
