use anyhow::{anyhow, Error, Result};
use ergo::cmd::make_api_key;
use futures::Future;

mod database;
mod tasks;

use database::{create_database, TestDatabase, TestUser};
use reqwest::header::HeaderMap;
use uuid::Uuid;

#[actix_rt::test]
async fn smoke_test() {
    run_app_test(|app| async move {
        let response = app
            .admin_user_client
            .get(format!("{}/tasks", app.base_url))
            .send()
            .await?;

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
    /// The ID of the precreated organization.
    pub org_id: Uuid,
    pub admin_user: TestUser,
    /// A client that automatically authenticates as the admin user.
    pub admin_user_client: reqwest::Client,
    pub address: String,
    pub base_url: String,
}

async fn start_app(database: TestDatabase, org_id: Uuid, admin_user: TestUser) -> Result<TestApp> {
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

    let mut headers = HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        format!("Bearer {}", admin_user.api_key).parse().unwrap(),
    );

    Ok(TestApp {
        database,
        org_id,
        admin_user,
        admin_user_client: reqwest::ClientBuilder::new()
            .default_headers(headers)
            .build()
            .expect("Building client"),
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
    let (database, _, _) = create_database().await.expect("Creating database");
    f(database).await.unwrap();
}

pub async fn run_app_test<F, R, RV, RE>(f: F) -> ()
where
    F: FnOnce(TestApp) -> R,
    R: Future<Output = Result<RV, RE>>,
    RV: Send,
    RE: Send + std::fmt::Debug,
{
    let (database, org_id, admin_user) = create_database().await.expect("Creating database");
    let app = start_app(database, org_id, admin_user)
        .await
        .expect("Starting app");
    f(app).await.unwrap();
}

impl TestApp {
    pub async fn add_user_with_password(
        &self,
        org_id: &Uuid,
        name: &str,
        password: Option<&str>,
    ) -> Result<TestUser> {
        if password.is_some() {
            todo!("Password support will be implemented once the API supports creating users");
        }

        let user_id = Uuid::new_v4();
        let mut conn = self.database.pool.acquire().await?;

        sqlx::query!(
            "INSERT INTO user_entity_ids (user_entity_id) VALUES ($1)",
            user_id
        )
        .execute(&mut conn)
        .await?;

        sqlx::query!(
            r##"INSERT INTO users (user_id, active_org_id, name, email, password_hash) VALUES
                ($1, $2, $3, $4, $5)"##,
            user_id,
            org_id,
            name,
            "test_user@example.com",
            ""
        )
        .execute(&mut conn)
        .await?;

        let key = make_api_key::make_key(&mut conn, org_id, Some(&user_id), false, None).await?;

        Ok(TestUser {
            user_id,
            org_id: org_id.clone(),
            password: None,
            api_key: key,
        })
    }

    pub async fn add_user(&self, org_id: &Uuid, name: &str) -> Result<TestUser> {
        self.add_user_with_password(org_id, name, None).await
    }
}
