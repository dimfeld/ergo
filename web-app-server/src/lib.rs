use actix_web::{
    get, http::StatusCode, web, web::Data, App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use config::Config;
use serde::Serialize;
use sqlx::{query, query_as};
use thiserror::Error;
use tracing_actix_web::TracingLogger;
use vault::{execute, VaultPostgresPool, VaultPostgresPoolOptions};

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

pub struct AppState {
    pg: VaultPostgresPool<()>,
}

pub fn app_data(config: Config) -> Result<Data<AppState>, std::io::Error> {
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

pub fn scope(app_data: &Data<AppState>, root: &str) -> actix_web::Scope {
    web::scope(root)
        .app_data(app_data.clone())
        .service(test)
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
