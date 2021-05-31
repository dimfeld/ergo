use anyhow::{anyhow, Error, Result};
use futures::Future;

mod database;
mod tasks;

use database::{create_database, TestDatabase};

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

pub struct TestApp {
    pub database: TestDatabase,
    pub address: String,
    pub base_url: String,
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

pub async fn run_database_test<F, R, RV, RE>(f: F) -> ()
where
    F: FnOnce(TestDatabase) -> R,
    R: Future<Output = Result<RV, RE>>,
    RV: Send,
    RE: Send + std::fmt::Debug,
{
    let database = create_database().await.expect("Creating database");
    f(database).await.unwrap();
}

pub async fn run_app_test<F, R, RV, RE>(f: F) -> ()
where
    F: FnOnce(TestApp) -> R,
    R: Future<Output = Result<RV, RE>>,
    RV: Send,
    RE: Send + std::fmt::Debug,
{
    let database = create_database().await.expect("Creating database");
    let app = start_app(database).await.expect("Starting app");
    f(app).await.unwrap();
}
