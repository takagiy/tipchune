use actix_web::{web, App, HttpServer};
use routes::sock;

mod routes;
mod services;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(web::scope("/sock").configure(sock::route)))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}
