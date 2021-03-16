use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use hashicorp_vault::client::VaultClient;
use std::sync::{Arc, RwLock};
use vault_postgres::VaultPostgresPool;

async fn health() -> impl Responder {
    HttpResponse::Ok().finish()
}

#[derive(Debug)]
pub struct Config {
    pub address: String,
    pub port: u16,
    pub vault_client: Arc<RwLock<VaultClient<()>>>,

    pub database: Option<String>,
    pub database_host: String,
    pub database_role: Option<String>,
}

struct AppState {
    pg: Arc<VaultPostgresPool>,
}

pub fn new(config: Config) -> std::io::Result<actix_web::dev::Server> {
    let Config {
        address,
        port,
        vault_client,
        database,
        database_host,
        database_role,
    } = config;

    let pg_pool = VaultPostgresPool::new(
        16,
        database_host,
        database.unwrap_or_else(|| "ergo".to_string()),
        database_role.unwrap_or_else(|| "ergo_web".to_string()),
        vault_client.clone(),
    )
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let app_state = web::Data::new(AppState { pg: pg_pool });

    let server = HttpServer::new(move || {
        App::new()
            .data(app_state.clone())
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
