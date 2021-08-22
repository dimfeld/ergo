use crate::{
    auth::middleware::AuthenticateMiddlewareFactory,
    error::Result,
    service_config::DatabaseConfiguration,
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

use std::{env, net::TcpListener, path::PathBuf};

use actix_files::NamedFile;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{
    dev::Server,
    web::{self, PathConfig},
    App, HttpServer,
};
use ergo_graceful_shutdown::GracefulShutdownConsumer;
use tracing::{event, Level};
use tracing_actix_web::TracingLogger;

pub struct Config<'a> {
    pub bind_address: Option<String>,
    pub bind_port: u16,
    pub database: DatabaseConfiguration,
    pub redis_url: Option<String>,
    pub redis_queue_prefix: Option<String>,
    pub vault_approle: Option<&'a str>,

    pub immediate_actions: bool,
    pub immediate_inputs: bool,
    pub no_drain_queues: bool,
    pub shutdown: GracefulShutdownConsumer,
}

pub async fn start<'a>(config: Config<'a>) -> Result<(Server, String, u16)> {
    let Config {
        bind_port,
        bind_address,
        database,
        redis_url,
        redis_queue_prefix,
        vault_approle,
        immediate_inputs,
        immediate_actions,
        no_drain_queues,
        shutdown,
    } = config;

    let bind_address = bind_address.unwrap_or_else(|| "127.0.0.1".to_string());
    let listener = TcpListener::bind(&format!("{}:{}", bind_address, bind_port))?;
    let bound_port = listener.local_addr()?.port();

    let vault_client =
        ergo_database::vault::from_env(vault_approle.unwrap_or("AIO_SERVER"), shutdown.clone())
            .await;
    tracing::info!(
        "Vault mode {}",
        vault_client
            .as_ref()
            .map(|_| "enabled")
            .unwrap_or("disabled")
    );

    let web_pg_pool =
        crate::service_config::web_pg_pool(shutdown.clone(), &vault_client, database.clone())
            .await?;
    let backend_pg_pool =
        crate::service_config::backend_pg_pool(shutdown.clone(), &vault_client, database).await?;

    let redis_pool = ergo_database::RedisPool::new(redis_url, redis_queue_prefix)?;

    let input_queue = InputQueue::new(redis_pool.clone());
    let action_queue = ActionQueue::new(redis_pool.clone());

    let notifications = crate::notifications::NotificationManager::new(
        backend_pg_pool.clone(),
        redis_pool.clone(),
        shutdown.clone(),
    )?;

    let web_app_data = crate::web_app_server::app_data(web_pg_pool.clone());
    let backend_app_data = crate::backend_data::app_data(
        backend_pg_pool.clone(),
        notifications.clone(),
        input_queue.clone(),
        action_queue.clone(),
        immediate_inputs,
        immediate_actions,
    )?;

    let _queue_drain = if no_drain_queues {
        None
    } else {
        Some(crate::tasks::queue_drain_runner::AllQueuesDrain::new(
            input_queue.clone(),
            action_queue.clone(),
            backend_pg_pool.clone(),
            redis_pool.clone(),
            shutdown.clone(),
        )?)
    };

    let _input_runner = TaskExecutor::new(TaskExecutorConfig {
        redis_pool: redis_pool.clone(),
        pg_pool: backend_pg_pool.clone(),
        shutdown: shutdown.clone(),
        notifications: Some(notifications.clone()),
        max_concurrent_jobs: None,
        immediate_actions,
    })?;

    let _action_runner = ActionExecutor::new(ActionExecutorConfig {
        redis_pool: redis_pool.clone(),
        pg_pool: backend_pg_pool.clone(),
        shutdown: shutdown.clone(),
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

    let serve_dir = env::var("SERVE_DIR").ok().unwrap_or_else(String::new);

    let server = HttpServer::new(move || {
        let identity = IdentityService::new(
            CookieIdentityPolicy::new(&cookie_signing_key)
                .http_only(true)
                .secure(true)
                .same_site(actix_web::cookie::SameSite::Strict),
        );

        let mut app = App::new().service(
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
        );

        if !serve_dir.is_empty() {
            let index_path = PathBuf::from(&serve_dir).join("index.html");
            app = app.service(
                actix_files::Files::new("/", &serve_dir)
                    .prefer_utf8(true)
                    .index_file("index.html")
                    .default_handler(
                        NamedFile::open(index_path).expect("index.html must exist in SERVE_DIR"),
                    ),
            );
        }

        app
    })
    .listen(listener)?
    .run();

    Ok((server, bind_address, bound_port))
}
