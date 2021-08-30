use anyhow::{anyhow, Result};
use ergo_api::{cmd::make_api_key, server::Server};
use ergo_database::object_id::{ActionCategoryId, OrgId, UserId};
use futures::Future;
use once_cell::sync::Lazy;

mod client;
pub mod database;
mod tasks;

pub use client::*;
pub use tasks::*;

use database::{create_database, DatabaseUser, TestDatabase};
// use proc_macro::TokenStream;
// use quote::quote;
use uuid::Uuid;

static TRACING: Lazy<()> = Lazy::new(|| {
    if std::env::var("TEST_LOG").is_ok() {
        ergo_api::tracing_config::configure("test", std::io::stdout);
    } else {
        ergo_api::tracing_config::configure("test", std::io::sink);
    }
});

pub struct TestUser {
    pub org_id: OrgId,
    pub user_id: UserId,
    pub password: Option<String>,
    pub api_key: String,
    pub client: TestClient,
}

pub struct TestApp {
    pub database: TestDatabase,
    /// The ID of the precreated organization.
    pub org_id: OrgId,
    pub admin_user: TestUser,
    /// A client set to the base url of the server.
    pub client: TestClient,
    pub address: String,
    pub base_url: String,
    pub base_action_category: ActionCategoryId,
}

async fn start_app(
    database: TestDatabase,
    org_id: OrgId,
    admin_user: DatabaseUser,
) -> Result<TestApp> {
    let shutdown = ergo_graceful_shutdown::GracefulShutdown::new();
    let redis_key_prefix = Uuid::new_v4();
    let config = ergo_api::server::Config {
        database: database.config.clone(),
        bind_port: 0, // Bind to random port
        bind_address: Some("127.0.0.1".to_string()),
        redis_url: std::env::var("TEST_REDIS_URL").ok(),
        redis_queue_prefix: Some(redis_key_prefix.to_string()),
        vault_approle: None,
        immediate_inputs: true,
        immediate_actions: true,
        no_drain_queues: false,
        shutdown: shutdown.consumer(),
    };
    Lazy::force(&TRACING);
    let Server {
        server,
        bind_address,
        bind_port,
        tasks,
    } = ergo_api::server::start(config).await?;

    tokio::task::spawn(async move {
        let _tasks = tasks;
        let server_err = server.await;
        let shutdown_err = shutdown.shutdown().await;
        match (server_err, shutdown_err) {
            (Err(e), _) => Err(anyhow!(e)),
            (Ok(_), Err(e)) => Err(anyhow!(e)),
            (Ok(_), Ok(_)) => Ok(()),
        }
    });

    let base_url = format!("http://{}:{}/api", bind_address, bind_port);
    let client = TestClient {
        base: base_url.clone(),
        client: reqwest::ClientBuilder::new()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Building client"),
    };

    Ok(TestApp {
        database,
        org_id,
        admin_user: TestUser {
            org_id: admin_user.org_id,
            user_id: admin_user.user_id,
            password: admin_user.password,
            client: client.clone_with_api_key(admin_user.api_key.clone()),
            api_key: admin_user.api_key,
        },
        client,
        address: format!("{}:{}", bind_address, bind_port),
        base_action_category: admin_user.action_category_id,
        base_url,
    })
}

pub async fn run_database_test<F, R>(f: F) -> ()
where
    F: FnOnce(TestDatabase) -> R,
    R: Future<Output = Result<(), anyhow::Error>>,
{
    let (database, _, _) = create_database().await.expect("Creating database");
    f(database.clone()).await.unwrap();
    database.drop_db().await.expect("Cleaning up");
}

pub async fn run_app_test<F, R>(f: F) -> ()
where
    F: FnOnce(TestApp) -> R,
    R: Future<Output = Result<(), anyhow::Error>>,
{
    let (database, org_id, admin_user) = create_database().await.expect("Creating database");
    let app = start_app(database.clone(), org_id, admin_user)
        .await
        .expect("Starting app");
    f(app).await.unwrap();
    database.drop_db().await.expect("Cleaning up");
}

impl TestApp {
    pub async fn add_org(&self, name: &str) -> Result<OrgId> {
        let mut conn = self.database.pool.acquire().await?;
        let org_id = OrgId::new();

        sqlx::query!(
            r##"INSERT INTO orgs (org_id, name) VALUES ($1, $2)"##,
            &org_id.0,
            name
        )
        .execute(&mut conn)
        .await?;
        println!("Created org {}", org_id);
        Ok(org_id)
    }

    pub async fn add_user_with_password(
        &self,
        org_id: &OrgId,
        name: &str,
        password: Option<&str>,
    ) -> Result<TestUser> {
        if password.is_some() {
            todo!("Password support will be implemented once the API supports creating users");
        }

        let user_id = UserId::new();
        let mut conn = self.database.pool.acquire().await?;

        sqlx::query!(
            r##"INSERT INTO users (user_id, active_org_id, name, email, password_hash) VALUES
                ($1, $2, $3, $4, $5)"##,
            &user_id.0,
            &org_id.0,
            name,
            format!("test_user_{}@example.com", user_id),
            ""
        )
        .execute(&mut conn)
        .await?;

        let key = make_api_key::make_key(&mut conn, org_id, Some(&user_id), false, None).await?;

        println!("Org {} added user {}: {}", org_id, name, user_id);
        Ok(TestUser {
            user_id,
            org_id: org_id.clone(),
            password: None,
            client: self.client.clone_with_api_key(key.clone()),
            api_key: key,
        })
    }

    pub async fn add_user(&self, org_id: &OrgId, name: &str) -> Result<TestUser> {
        self.add_user_with_password(org_id, name, None).await
    }
}

// #[proc_macro_attribute]
// pub fn app_test(_: TokenStream, item: TokenStream) -> TokenStream {
//     let mut input = syn::parse_macro_input!(item as syn::ItemFn);
//     let attrs = &input.attrs;
//     let vis = &input.vis;
//     let sig = &mut input.sig;
//     let body = &input.block;
//     let mut has_test_attr = false;
//
//     for attr in attrs {
//         if attr.path.is_ident("test") {
//             has_test_attr = true;
//         }
//     }
//
//     if sig.asyncness.is_none() {
//         return syn::Error::new_spanned(
//             input.sig.fn_token,
//             "the async keyword is missing from the function declaration",
//         )
//         .to_compile_error()
//         .into();
//     }
//
//     sig.asyncness = None;
//
//     let missing_test_attr = if has_test_attr {
//         quote!()
//     } else {
//         quote!(#[test])
//     };
//
//     let appname = match sig.inputs.first() {
//         Some(syn::FnArg::Typed(syn::PatType { pat: p, .. })) => match &**p {
//             syn::Pat::Ident(p) => p.clone(),
//             _ => {
//                 return syn::Error::new_spanned(
//                     input.sig.fn_token,
//                     "first argument must be a TestApp",
//                 )
//                 .to_compile_error()
//                 .into()
//             }
//         },
//         _ => {
//             return syn::Error::new_spanned(input.sig.fn_token, "first argument must be a TestApp")
//                 .to_compile_error()
//                 .into();
//         }
//     };
//
//     // Remove all the arguments.
//     sig.inputs = syn::punctuated::Punctuated::new();
//
//     (quote! {
//         #missing_test_attr
//         #(#attrs)*
//         #vis #sig {
//             actix_rt::System::new()
//                 .block_on(async {
//                     let #appname = {
//                         let (database, org_id, admin_user) = ergo_database::create_database().await.expect("Creating database");
//                         crate::common::start_app(database, org_id, admin_user)
//                             .await
//                             .expect("Starting app");
//                     };
//
//                     #body
//                 })
//         }
//     })
//     .into()
// }
