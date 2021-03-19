use actix_web::{
    get, http::StatusCode, web, web::Data, App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use serde::Serialize;
use sqlx::{query, query_as};
use std::sync::{Arc, RwLock};
use thiserror::Error;
use tracing_actix_web::TracingLogger;
use vault::{execute, AppRoleVaultClient, VaultPostgresPool, VaultPostgresPoolOptions};

#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error(transparent)]
    DbError(#[from] vault::Error),

    #[error("SQL Error")]
    SqlError(#[from] sqlx::error::Error),
}

impl actix_web::error::ResponseError for Error {
    fn error_response(&self) -> HttpResponse<actix_web::dev::Body> {
        HttpResponse::InternalServerError().body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

async fn health() -> impl Responder {
    HttpResponse::Ok().finish()
}

#[derive(Serialize)]
struct TestRow {
    id: i64,
    value: String,
}

#[get("/test")]
async fn test(state: Data<AppState>) -> Result<HttpResponse, Error> {
    let results = query_as!(TestRow, "SELECT * FROM test")
        .fetch_all(execute!(state.pg))
        .await?;

    Ok(HttpResponse::Ok().json(results))
}

#[derive(Debug)]
pub struct Config {
    pub address: String,
    pub port: u16,
    pub vault_client: AppRoleVaultClient,

    pub database: Option<String>,
    pub database_host: String,
    pub database_role: Option<String>,

    pub shutdown: graceful_shutdown::GracefulShutdownConsumer,
}

struct AppState {
    pg: VaultPostgresPool<()>,
}

pub fn new(config: Config) -> std::io::Result<actix_web::dev::Server> {
    let Config {
        address,
        port,
        vault_client,
        database,
        database_host,
        database_role,
        shutdown,
    } = config;

    let pg_pool = VaultPostgresPool::new(VaultPostgresPoolOptions {
        max_connections: 16,
        host: database_host,
        database: database.unwrap_or_else(|| "ergo".to_string()),
        role: database_role.unwrap_or_else(|| "ergo_web".to_string()),
        vault_client: vault_client.clone(),
        shutdown,
    })
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let app_state = Data::new(AppState { pg: pg_pool });

    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger)
            .app_data(app_state.clone())
            .service(test)
            .route("/healthz", web::get().to(health))
    })
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
