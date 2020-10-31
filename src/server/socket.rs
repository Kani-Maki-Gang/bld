use crate::config::BldConfig;
use crate::persist::FileSystemDumpster;
use crate::run::Runner;
use crate::term;
use actix::prelude::*;
use actix_web_actors::ws;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::runtime::Runtime;

pub struct PipelineWebSocketServer {
    hb: Instant,
    is_pipeline_done: bool,
}

impl PipelineWebSocketServer {
    pub fn new() -> Self {
        Self {
            hb: Instant::now(),
            is_pipeline_done: false,
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
        if self.is_pipeline_done {
            ctx.close(None);
        }
        match msg {
            Ok(ws::Message::Text(text)) => {
                if let Some((name, src)) = parse_text(&text) {
                    std::thread::spawn(move || invoke_pipeline(name, src));
                }
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

fn parse_text(text: &str) -> Option<(String, String)> {
    match serde_json::from_str::<Value>(text) {
        Ok(message) => {
            let name = match message["name"].as_str() {
                Some(name) => {
                    let time = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("Time went backwards");
                    format!("{}-{}", name, time.as_nanos())
                }
                None => return None,
            };
            let src = match message["pipeline"].as_str() {
                Some(src) => src.to_string(),
                None => return None,
            };
            Some((name, src))
        }
        Err(_) => None,
    }
}

fn invoke_pipeline(name: String, src: String) {
    if let Ok(mut rt) = Runtime::new() {
        rt.block_on(async move {
            if let Ok(config) = BldConfig::load() {
                let path = {
                    let mut path = std::path::PathBuf::new();
                    path.push(config.local.logs);
                    path.push(name);
                    path.display().to_string()
                };
                let dumpster = match FileSystemDumpster::new(&path) {
                    Ok(d) => d,
                    Err(e) => {
                        let _ = term::print_error(&e.to_string());
                        return;
                    }
                };
                let dumpster = Arc::new(Mutex::new(dumpster));
                if let Err(e) = Runner::from_src(src, dumpster).await.await {
                    let _ = term::print_error(&format!("{}", e));
                }
            }
        });
    }
}
