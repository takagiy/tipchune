use actix::{
    clock::Instant, Actor, ActorContext, Addr, AsyncContext, Context, Handler, Message, Running,
    StreamHandler,
};
use actix_web_actors::ws;
use std::{sync::Arc, time::Duration};
use tipchune::primitive::{Block, Transaction};

pub struct WsConnectionManager {
    connections: Vec<Addr<WsConnection>>,
}

impl WsConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
        }
    }
}

impl Actor for WsConnectionManager {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Join {
    connection: Addr<WsConnection>,
}

impl Handler<Join> for WsConnectionManager {
    type Result = <Join as Message>::Result;

    fn handle(&mut self, msg: Join, ctx: &mut Self::Context) -> Self::Result {
        self.connections.push(msg.connection);
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Leave {
    connection: Addr<WsConnection>,
}

impl Handler<Leave> for WsConnectionManager {
    type Result = <Leave as Message>::Result;

    fn handle(&mut self, msg: Leave, ctx: &mut Self::Context) -> Self::Result {
        self.connections.retain(|conn| *conn != msg.connection);
    }
}

pub struct WsConnection {
    last_hb: Instant,
    manager: Addr<WsConnectionManager>,
}

impl WsConnection {
    pub fn new(manager: Addr<WsConnectionManager>) -> Self {
        Self {
            last_hb: Instant::now(),
            manager,
        }
    }

    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
        const TIMEOUT: Duration = Duration::from_secs(30);

        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.last_hb) > TIMEOUT {
                act.manager.do_send(Leave {
                    connection: ctx.address(),
                });
                ctx.stop();
                return;
            }

            ctx.ping(&[]);
        });
    }
}

impl Actor for WsConnection {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
        self.manager.do_send(Join {
            connection: ctx.address(),
        });
    }

    fn stopping(&mut self, ctx: &mut Self::Context) -> actix::Running {
        self.manager.do_send(Leave {
            connection: ctx.address(),
        });
        Running::Stop
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ForwardBlock {
    block: Arc<Block>,
}

impl Handler<ForwardBlock> for WsConnection {
    type Result = <ForwardBlock as Message>::Result;

    fn handle(&mut self, msg: ForwardBlock, ctx: &mut Self::Context) -> Self::Result {
        //TODO: Define schema and send message over Websocket
        ctx.text("forward block");
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ForwardTransaction {
    transaction: Arc<Transaction>,
}

impl Handler<ForwardTransaction> for WsConnection {
    type Result = <ForwardTransaction as Message>::Result;

    fn handle(&mut self, msg: ForwardTransaction, ctx: &mut Self::Context) -> Self::Result {
        //TODO: Define schema and send message over Websocket
        ctx.text("forward transaction");
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsConnection {
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let msg = match item {
            Err(_) => {
                ctx.stop();
                return;
            }
            Ok(msg) => msg,
        };

        match msg {
            ws::Message::Ping(msg) => {
                self.last_hb = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.last_hb = Instant::now();
            }
            ws::Message::Text(_text) => {
                //TODO: Define schema and parse message
            }
            ws::Message::Binary(_) => (),
            ws::Message::Close(reason) => {
                ctx.close(reason);
                ctx.stop();
            }
            ws::Message::Continuation(_) => {
                ctx.stop();
            }
            ws::Message::Nop => (),
        }
    }
}
