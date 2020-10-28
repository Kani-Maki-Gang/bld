use crate::run::Runner;
use crate::term;
use crate::persist::FileSystemDumpster;
use actix::fut::WrapFuture;
use actix::prelude::*;
use actix_web_actors::ws;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};

pub struct PipelineWebSocketServer {
    hb: Instant,
    is_processing: bool,
}

impl PipelineWebSocketServer {
    pub fn new() -> Self {
        Self {
            hb: Instant::now(),
            is_processing: false,
        }
    }

    fn heartbeat(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(Duration::from_secs(5), |act, ctx| {
            if Instant::now().duration_since(act.hb) > Duration::from_secs(10) {
                println!("Websocket heartbeat failed, disconnecting!");
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }
}

impl Actor for PipelineWebSocketServer {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.heartbeat(ctx);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for PipelineWebSocketServer {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        if self.is_processing {
            ctx.close(None);
        }
        match msg {
            Ok(ws::Message::Text(text)) => {
                let content = String::from(text);
                let ft = async {
                    let dumpster = match FileSystemDumpster::new("") {
                        Ok(d) => d,
                        Err(e) => {
                            let _ = term::print_error(&e.to_string());
                            return;
                        }
                    };
                    let dumpster = Arc::new(Mutex::new(dumpster));
                    if let Err(e) = Runner::from_src(content, dumpster).await.await {
                        let _ = term::print_error(&format!("{}", e));
                    }
                }
                .into_actor(self);
                ctx.wait(ft);
                self.is_processing = true;
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
