use crate::{
    database::{PostgresPool, VaultPostgresPool, VaultPostgresPoolOptions},
    error::Error,
    service_config::Config,
};

use actix_web::{get, web, web::Data, App, HttpResponse, HttpServer, Responder};
use serde::Serialize;
use sqlx::query_as;
use tracing_actix_web::TracingLogger;

async fn health() -> impl Responder {
    HttpResponse::Ok().finish()
}

#[derive(Serialize)]
struct TestRow {
    id: i64,
    value: String,
}

pub struct AppState {
    pg: PostgresPool,
}

pub type AppStateData = Data<AppState>;

pub fn app_data(config: Config) -> Result<AppStateData, std::io::Error> {
    let pg_pool = VaultPostgresPool::new(VaultPostgresPoolOptions {
        max_connections: 16,
        host: config.database_host,
        database: config.database.unwrap_or_else(|| "ergo".to_string()),
        role: config
            .database_role
            .unwrap_or_else(|| "ergo_web".to_string()),
        vault_client: config.vault_client.clone(),
        shutdown: config.shutdown,
    })
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    Ok(Data::new(AppState { pg: pg_pool }))
}

pub fn scope(app_data: &AppStateData, root: &str) -> actix_web::Scope {
    web::scope(root)
        .app_data(app_data.clone())
        .route("/healthz", web::get().to(health))
}

pub fn new(address: String, port: u16, config: Config) -> std::io::Result<actix_web::dev::Server> {
    let data = app_data(config)?;
    let server = HttpServer::new(move || App::new().wrap(TracingLogger).service(scope(&data, "")))
        .bind(format!("{}:{}", address, port))?
        .run();

    Ok(server)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
