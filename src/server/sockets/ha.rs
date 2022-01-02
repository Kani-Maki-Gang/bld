#![allow(dead_code)]

use crate::config::BldConfig;
use crate::server::User;
use actix::prelude::*;
use actix_web::{error::ErrorUnauthorized, web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use tracing::debug;
use std::time::{Duration, Instant};

type StdResult<T, V> = std::result::Result<T, V>;

pub struct HighAvailSocket {
    _config: web::Data<BldConfig>,
    hb: Instant,
}

impl HighAvailSocket {
    pub fn new(config: web::Data<BldConfig>) -> Self {
        Self {
            _config: config,
            hb: Instant::now(),
        }
    }

    fn heartbeat(act: &Self, ctx: &mut <Self as Actor>::Context) {
        if Instant::now().duration_since(act.hb) > Duration::from_secs(10) {
            println!("High availability websocket heartbeat failed. disconnecting!");
            ctx.stop();
            return;
        }
        ctx.ping(b"");
    }
}

impl Actor for HighAvailSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(Duration::from_secs(1), |act, ctx| {
            HighAvailSocket::heartbeat(act, ctx);
        });
    }
}

impl StreamHandler<StdResult<ws::Message, ws::ProtocolError>> for HighAvailSocket {
    fn handle(&mut self, msg: StdResult<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(txt)) => {
                println!("{}", txt);
            }
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

pub async fn ws_high_avail(
    user: Option<User>,
    req: HttpRequest,
    stream: web::Payload,
    config: web::Data<BldConfig>,
) -> StdResult<HttpResponse, Error> {
    debug!("starting high avail web socket");
    if user.is_none() {
        return Err(ErrorUnauthorized(""));
    }
    ws::start(HighAvailSocket::new(config), &req, stream)
}
