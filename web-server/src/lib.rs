use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use hashicorp_vault::client::VaultClient;
use std::sync::{Arc, RwLock};

async fn health() -> impl Responder {
    HttpResponse::Ok().finish()
}

#[derive(Debug)]
pub struct Config {
    pub address: String,
    pub port: u16,
    pub vault_client: Arc<RwLock<VaultClient<()>>>,
}

struct AppState {
    vault_client: Arc<RwLock<VaultClient<()>>>,
}

pub fn new(config: Config) -> std::io::Result<actix_web::dev::Server> {
    let Config {
        address,
        port,
        vault_client,
    } = config;

    let app_state = web::Data::new(AppState { vault_client });

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
