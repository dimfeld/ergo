use crate::{error::Result, routes};

use std::{env, net::TcpListener, path::PathBuf};

use actix_files::NamedFile;
use actix_identity::IdentityMiddleware;
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::{
    web::{self, PathConfig},
    App, HttpServer,
};
use ergo_auth::middleware::AuthenticateMiddlewareFactory;
use ergo_database::DatabaseConfiguration;
use ergo_graceful_shutdown::GracefulShutdownConsumer;
use ergo_notifications::NotificationManager;
use ergo_tasks::{
    actions::{
        dequeue::{ActionExecutor, ActionExecutorConfig},
        queue::ActionQueue,
    },
    inputs::{
        dequeue::{TaskExecutor, TaskExecutorConfig},
        queue::InputQueue,
    },
    periodic::monitor_missing_periodic_triggers,
    queue_drain_runner::AllQueuesDrain,
};
use tracing::{event, info, Level};
use tracing_actix_web::TracingLogger;

pub struct Config {
    pub bind_address: Option<String>,
    pub bind_port: u16,
    pub database: DatabaseConfiguration,
    pub redis_url: Option<String>,
    pub redis_queue_prefix: Option<String>,

    pub no_drain_queues: bool,
    pub shutdown: GracefulShutdownConsumer,
}

/// Tasks that run within the server. Keep this object alive for the duration of the process.
pub struct ServerTasks {
    notification_manager: NotificationManager,
    queue_drain: Option<AllQueuesDrain>,
    input_runner: TaskExecutor,
    action_runner: ActionExecutor,
    periodic_task_monitor: tokio::task::JoinHandle<()>,
}

pub struct Server {
    pub server: actix_web::dev::Server,
    pub bind_address: String,
    pub bind_port: u16,
    pub tasks: ServerTasks,
}

pub async fn start(config: Config) -> Result<Server> {
    let Config {
        bind_port,
        bind_address,
        database,
        redis_url,
        redis_queue_prefix,
        no_drain_queues,
        shutdown,
    } = config;

    let bind_address = bind_address.unwrap_or_else(|| "127.0.0.1".to_string());
    let listener = TcpListener::bind(&format!("{}:{}", bind_address, bind_port))?;
    let bind_port = listener.local_addr()?.port();

    let web_pg_pool = crate::service_config::web_pg_pool(&database).await?;
    let backend_pg_pool = crate::service_config::backend_pg_pool(&database).await?;

    let redis_pool = ergo_database::RedisPool::new(redis_url, redis_queue_prefix.clone())?;

    let input_queue = InputQueue::new(redis_pool.clone());
    let action_queue = ActionQueue::new(redis_pool.clone());

    input_queue.start_scheduled_jobs_enqueuer(shutdown.clone());
    action_queue.start_scheduled_jobs_enqueuer(shutdown.clone());

    let mut notifications = ergo_notifications::NotificationManager::new(
        backend_pg_pool.clone(),
        redis_pool.clone(),
        shutdown.clone(),
    )?;

    notifications.start_task_queue_loop()?;

    let web_app_data =
        crate::web_app_server::app_data(web_pg_pool.clone(), redis_queue_prefix.clone());
    let backend_app_data = crate::backend_data::app_data(
        backend_pg_pool.clone(),
        notifications.clone(),
        input_queue,
        action_queue,
        redis_queue_prefix.clone(),
    )?;

    let queue_drain = if no_drain_queues {
        None
    } else {
        info!("Starting postgres queue drain");
        Some(ergo_tasks::queue_drain_runner::AllQueuesDrain::new(
            backend_pg_pool.clone(),
            redis_pool.clone(),
            shutdown.clone(),
        )?)
    };

    let periodic_task_monitor = monitor_missing_periodic_triggers(
        shutdown.clone(),
        backend_pg_pool.clone(),
        redis_queue_prefix.clone(),
        None,
    );

    let input_runner = TaskExecutor::new(TaskExecutorConfig {
        redis_pool: redis_pool.clone(),
        pg_pool: backend_pg_pool.clone(),
        shutdown: shutdown.clone(),
        notifications: Some(notifications.clone()),
        max_concurrent_jobs: None,
    })?;

    let action_runner = ActionExecutor::new(ActionExecutorConfig {
        redis_pool,
        pg_pool: backend_pg_pool,
        shutdown: shutdown.clone(),
        notifications: Some(notifications.clone()),
        max_concurrent_jobs: None,
    })?;

    let cookie_signing_key = env::var("COOKIE_SIGNING_KEY")
        .ok()
        .unwrap_or_else(|| {
            event!(
                Level::WARN,
                "Using default cookie signing key. Set COOKIE_SIGNING_KEY environment variable to a 64-byte string to set it"
            );

            "wpvuwm4pvoane;bwn40s;wmvlscvG@sV".to_string()
        })
        .into_bytes();

    let serve_dir = env::var("SERVE_DIR").ok().unwrap_or_default();

    let server = HttpServer::new(move || {
        let sessions = SessionMiddleware::new(
            CookieSessionStore::default(),
            actix_web::cookie::Key::from(&cookie_signing_key),
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
                .wrap(IdentityMiddleware::default())
                .wrap(sessions)
                .wrap(TracingLogger::default())
                .configure(routes::accounts::config)
                .configure(routes::actions::config)
                .configure(routes::action_categories::config)
                .configure(routes::inputs::config)
                .configure(routes::status::config)
                .configure(routes::tasks::config),
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

    Ok(Server {
        server,
        bind_address,
        bind_port,
        tasks: ServerTasks {
            notification_manager: notifications,
            queue_drain,
            input_runner,
            action_runner,
            periodic_task_monitor,
        },
    })
}
