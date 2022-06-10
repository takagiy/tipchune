use actix_web::web::{self, ServiceConfig};

mod sock;

pub fn route(cfg: &mut ServiceConfig) {
    cfg.service(web::scope("/sock").configure(sock::route));
}
