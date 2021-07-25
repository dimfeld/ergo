use actix_web::{web, HttpResponse, Responder};

async fn health() -> impl Responder {
    HttpResponse::Ok().finish()
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.route("/healthz", web::get().to(health));
}
