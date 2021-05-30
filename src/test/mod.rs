use anyhow::Result;
use futures::Future;
use sqlx::{migrate::Migrator, Connection, PgConnection};
use uuid::Uuid;

use crate::{
    database::PostgresPool,
    service_config::{database_configuration_from_env, DatabaseConfiguration},
};

const BASE_MIGRATIONS: &'static str = "./migrations";
const DATA_MIGRATIONS: &'static str = "./src/test/migrations";

pub struct TestDatabase {
    pub config: DatabaseConfiguration,
    pub name: String,
}

pub struct TestApp {
    pub database: TestDatabase,
    pub address: String,
}

async fn run_migrations(conn: &mut PgConnection, dir: &str) -> Result<()> {
    let migrator = Migrator::new(std::path::Path::new(dir)).await?;
    migrator.run(conn).await?;
    Ok(())
}

async fn create_database() -> Result<TestDatabase> {
    // 1. Create the database.
    let mut config = database_configuration_from_env()?;
    let db_name = format!("ergo-test-{}", Uuid::new_v4());
    config.database = db_name;

    let global_conn = PgConnection::connect(&std::env::var("DATABASE_URL")?).await?;
    sqlx::query(&format!("CREATE DATABASE {}", db_name))
        .execute(&mut global_conn)
        .await?;

    let conn_str = ""; // TODO
    let mut database_conn = PgConnection::connect(conn_str).await?;
    run_migrations(&mut database_conn, BASE_MIGRATIONS);
    run_migrations(&mut database_conn, DATA_MIGRATIONS);

    Ok(TestDatabase {
        name: db_name,
        config,
    })
}

async fn start_app(database: TestDatabase) -> Result<TestApp> {
    let shutdown = crate::graceful_shutdown::GracefulShutdown::new();
    let database = create_database().await?;
    let config = crate::server::Config {
        database: database.config,
        bind_port: 0, // Bind to randome port
        bind_address: Some("127.0.0.1".to_string()),
        redis_url: None,
        vault_approle: None,

        immediate_inputs: false,
        immediate_actions: false,
        no_drain_queues: false,
        shutdown: shutdown.consumer(),
    };
    let (server, addr, port) = crate::server::start(config).await?;

    tokio::task::spawn(async move {
        let server_err = server.await?;
        let shutdown_err = shutdown.shutdown().await.ok();
        match (server_err, shutdown_err) {
            (Err(e), _) => Err(e),
            (Ok(_), Err(e)) => Err(e),
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

pub async fn run_app_test<F, R, RT>(f: F) -> RT
where
    F: FnOnce(TestApp) -> R,
    R: Future<Output = RT>,
    RT: Send,
{
    let database = create_database().await.expect("Creating database");
    let app = start_app(database).await.expect("Starting app");
    f(app).await
}
