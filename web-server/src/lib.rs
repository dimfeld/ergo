use actix_web::{web, App, HttpRequest, HttpServer, Responder};

async fn health() -> impl Responder {
    ""
}

pub fn new() -> std::io::Result<actix_web::dev::Server> {
    let bind_addr = "127.0.0.1";
    let bind_port = "6543";
    let server = HttpServer::new(|| App::new().route("/healthz", web::get().to(health)))
        .bind(format!("{}:{}", bind_addr, bind_port))?
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
