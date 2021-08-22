#![allow(dead_code, unused_imports, unused_variables)] // Remove this once the basic application is up and working
use crate::{error::Error, vault::VaultClientTokenData};

use actix_web::{get, web, web::Data, App, HttpResponse, HttpServer, Responder, Scope};
use ergo_database::{PostgresPool, VaultPostgresPool, VaultPostgresPoolOptions};
use serde::Serialize;
use sqlx::query_as;
use tracing_actix_web::TracingLogger;

#[derive(Serialize)]
struct TestRow {
    id: i64,
    value: String,
}

pub struct AppState {
    pub pg: PostgresPool,
}

pub type AppStateData = Data<AppState>;

pub fn app_data(pg: VaultPostgresPool) -> AppStateData {
    Data::new(AppState { pg })
}

pub fn config(cfg: &mut web::ServiceConfig) {}

// pub fn new_server(
//     address: String,
//     port: u16,
//     pg_pool: VaultPostgresPool,
// ) -> std::io::Result<actix_web::dev::Server> {
//     let data = app_data(pg_pool);
//     let server = HttpServer::new(move || {
//         App::new()
//             .wrap(TracingLogger::default())
//             .configure(scope(&data, ""))
//     })
//     .bind(format!("{}:{}", address, port))?
//     .run();
//
//     Ok(server)
// }
