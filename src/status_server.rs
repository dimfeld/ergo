use actix_web::{web, HttpResponse, Responder};

async fn health() -> impl Responder {
    HttpResponse::Ok().finish()
}

pub fn scope(root: &str) -> actix_web::Scope {
    web::scope(root).route("/healthz", web::get().to(health))
}
