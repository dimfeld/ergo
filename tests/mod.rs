use anyhow::{anyhow, Result};
use futures::Future;
use sqlx::{postgres::PgConnectOptions, ConnectOptions};
use uuid::Uuid;

use std::env;

use ergo::{database::PostgresPool, service_config::DatabaseConfiguration};

mod tasks;

#[actix_rt::test]
async fn smoke_test() {
    run_app_test(|_app| async { Ok::<(), ()>(()) }).await;
}

pub struct TestDatabase {
    pub config: DatabaseConfiguration,
    pub name: String,
}

pub struct TestApp {
    pub database: TestDatabase,
    pub address: String,
}

async fn create_database() -> Result<TestDatabase> {
    dotenv::dotenv().ok();
    let host = std::env::var("TEST_DATABASE_HOST")
        .or_else(|_| env::var("DATABASE_HOST"))
        .unwrap_or_else(|_| "localhost".to_string());
    let port = std::env::var("TEST_DATABASE_PORT")
        .or_else(|_| env::var("DATABASE_PORT"))
        .map_err(|e| anyhow!(e))
        .and_then(|val| val.parse::<u16>().map_err(|e| anyhow!(e)))
        .unwrap_or(5432);
    let user = env::var("TEST_DATABASE_USER").unwrap_or_else(|_| "postgres".to_string());
    let password = env::var("TEST_DATABASE_PASSWORD").unwrap_or_else(|_| "".to_string());

    let config = DatabaseConfiguration {
        database: format!("ergo-test-{}", Uuid::new_v4()),
        host,
        port,
    };

    let mut global_conn = PgConnectOptions::new()
        .port(port)
        .host(&config.host)
        .username(&user)
        .password(&password)
        .connect()
        .await?;

    sqlx::query(&format!(r##"CREATE DATABASE "{}";"##, config.database))
        .execute(&mut global_conn)
        .await?;
    drop(global_conn);

    let mut database_conn = PgConnectOptions::new()
        .port(port)
        .host(&config.host)
        .database(&config.database)
        .username(&user)
        .password(&password)
        .connect()
        .await?;
    sqlx::migrate!("./migrations")
        .run(&mut database_conn)
        .await?;

    Ok(TestDatabase {
        name: config.database.clone(),
        config,
    })
}

async fn start_app(database: TestDatabase) -> Result<TestApp> {
    let shutdown = ergo::graceful_shutdown::GracefulShutdown::new();
    let config = ergo::server::Config {
        database: database.config.clone(),
        bind_port: 0, // Bind to random port
        bind_address: Some("127.0.0.1".to_string()),
        redis_url: None,
        vault_approle: None,

        immediate_inputs: false,
        immediate_actions: false,
        no_drain_queues: false,
        shutdown: shutdown.consumer(),
    };
    let (server, addr, port) = ergo::server::start(config).await?;

    tokio::task::spawn(async move {
        let server_err = server.await;
        let shutdown_err = shutdown.shutdown().await;
        match (server_err, shutdown_err) {
            (Err(e), _) => Err(anyhow!(e)),
            (Ok(_), Err(e)) => Err(anyhow!(e)),
            (Ok(_), Ok(_)) => Ok(()),
        }
    });

    Ok(TestApp {
        database,
        address: format!("{}:{}", addr, port),
    })
}

pub async fn run_database_test<F, R, RT>(f: F) -> RT
where
    F: FnOnce(TestDatabase) -> R,
    R: Future<Output = RT>,
    RT: Send,
{
    let database = create_database().await.expect("Creating database");
    f(database).await
}

pub async fn run_app_test<F, R, RT>(f: F) -> ()
where
    F: FnOnce(TestApp) -> R,
    R: Future<Output = Result<(), RT>>,
    RT: Send + std::fmt::Debug,
{
    let database = create_database().await.expect("Creating database");
    let app = start_app(database).await.expect("Starting app");
    f(app).await.unwrap()
}
