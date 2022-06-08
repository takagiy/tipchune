use actix::{Actor, Addr};
use actix_web::{
    get,
    web::{Data, Payload, ServiceConfig},
    Error, HttpRequest, HttpResponse,
};
use actix_web_actors::ws;

use crate::services::websocket::{WsConnection, WsConnectionManager};

#[get("/")]
async fn start_connection(
    req: HttpRequest,
    stream: Payload,
    connection_manager: Data<Addr<WsConnectionManager>>,
) -> Result<HttpResponse, Error> {
    let connection = WsConnection::new(connection_manager.as_ref().clone());
    ws::start(connection, &req, stream)
}

pub fn route(cfg: &mut ServiceConfig) {
    let connection_manager = WsConnectionManager::new().start();
    cfg.app_data(connection_manager);
    cfg.service(start_connection);
}
