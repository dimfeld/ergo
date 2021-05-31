use anyhow::{anyhow, Error, Result};
use futures::Future;
use sqlx::{postgres::PgConnectOptions, ConnectOptions, Executor};
use uuid::Uuid;

use std::env;

use ergo::{database::PostgresPool, service_config::DatabaseConfiguration};

mod tasks;

#[actix_rt::test]
async fn smoke_test() {
    run_app_test(|app| async move {
        let response = reqwest::get(format!("{}/healthz", app.base_url)).await?;
        assert_eq!(
            response.status().as_u16(),
            200,
            "response status code should be 200"
        );
        Ok::<(), Error>(())
    })
    .await;
}

pub struct TestDatabase {
    pub config: DatabaseConfiguration,
    pub name: String,
}

pub struct TestApp {
    pub database: TestDatabase,
    pub address: String,
    pub base_url: String,
}

fn password_sql(role: &str) -> String {
    if let Ok(pwd) = std::env::var(&format!("DATABASE_ROLE_{}_PASSWORD", role)) {
        let pwd = pwd.replace('\\', r##"\\"##).replace('\'', r##"\'"##);
        format!("LOGIN PASSWORD '{}'", pwd)
    } else {
        String::new()
    }
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

    sqlx::query(&format!(r##"CREATE DATABASE "{}""##, config.database))
        .execute(&mut global_conn)
        .await?;

    // The roles are global, but need to be set up. The migrations normally handle this but for
    // tests we need to make sure that the passwords are set.
    let roles_query = format!(
        r##"
DO $$BEGIN
  CREATE ROLE ergo_user INHERIT;
  EXCEPTION WHEN duplicate_object THEN NULL;
END; $$;

DO $$BEGIN
  CREATE ROLE ergo_web NOINHERIT IN ROLE ergo_user {web_password};
  EXCEPTION WHEN duplicate_object THEN NULL;
END; $$;

DO $$BEGIN
  CREATE ROLE ergo_backend NOINHERIT IN ROLE ergo_user {backend_password};
  EXCEPTION WHEN duplicate_object THEN NULL;
END; $$;

DO $$BEGIN
  CREATE ROLE ergo_enqueuer NOINHERIT IN ROLE ergo_user {enqueuer_password};
  EXCEPTION WHEN duplicate_object THEN NULL;
END; $$;
            "##,
        web_password = password_sql("WEB"),
        backend_password = password_sql("BACKEND"),
        enqueuer_password = password_sql("ENQUEUER"),
    );

    global_conn.execute(roles_query.as_str()).await?;
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
        base_url: format!("http://{}:{}/api", addr, port),
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
