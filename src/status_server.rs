use actix_web::{web, HttpResponse, Responder};
use tracing::{event, instrument, Level};

#[instrument(name = "health")]
async fn health() -> impl Responder {
    event!(Level::INFO, "health");
    HttpResponse::Ok().finish()
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("/healthz", web::get().to(health));
}
