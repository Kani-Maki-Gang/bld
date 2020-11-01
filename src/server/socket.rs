use crate::config::BldConfig;
use crate::persist::{FileLogger, FileScanner, Scanner};
use crate::run::Runner;
use crate::term;
use actix::prelude::*;
use actix_web_actors::ws;
use serde_json::Value;
use std::io::{self, Error, ErrorKind};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::runtime::Runtime;

pub struct PipelineWebSocketServer {
    hb: Instant,
    scanner: Option<FileScanner>,
    is_pipeline_done: bool,
}

impl PipelineWebSocketServer {
    pub fn new() -> Self {
        Self {
            hb: Instant::now(),
            scanner: None,
            is_pipeline_done: false,
        }
    }

    fn heartbeat(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(Duration::from_secs(1), |act, ctx| {
            if Instant::now().duration_since(act.hb) > Duration::from_secs(10) {
                println!("Websocket heartbeat failed, disconnecting!");
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }

    fn scan(&mut self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(Duration::from_secs(1), |act, ctx| {
            if let Some(scanner) = act.scanner.as_mut() {
                let content = scanner.fetch();
                for line in content.iter() {
                    ctx.text(line);
                }
            }
        });
    }
}

impl Actor for PipelineWebSocketServer {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.heartbeat(ctx);
        self.scan(ctx);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for PipelineWebSocketServer {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        if self.is_pipeline_done {
            ctx.close(None);
        }
        match msg {
            Ok(ws::Message::Text(txt)) => {
                if let Some((path, src)) = parse_text(&txt) {
                    if let Ok((logger, scanner)) = create_ds(&path) {
                        self.scanner = Some(scanner);
                        std::thread::spawn(move || invoke_pipeline(src, logger));
                    }
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
    if let Ok(config) = BldConfig::load() {
        if let Ok(message) = serde_json::from_str::<Value>(text) {
            let name = match message["name"].as_str() {
                Some(name) => {
                    let time = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("Time went backwards");
                    format!("{}-{}", name, time.as_nanos())
                }
                None => return None,
            };
            let path = {
                let mut path = std::path::PathBuf::new();
                path.push(config.local.logs);
                path.push(name);
                path.display().to_string()
            };
            let src = match message["pipeline"].as_str() {
                Some(src) => src.to_string(),
                None => return None,
            };
            return Some((path, src));
        }
    }
    None
}

fn create_ds(path: &str) -> io::Result<(FileLogger, FileScanner)> {
    let logger = match FileLogger::new(&path) {
        Ok(logger) => logger,
        Err(_) => return Err(Error::new(ErrorKind::Other, "could not create fs logger")),
    };
    let scanner = match FileScanner::new(path) {
        Ok(scav) => scav,
        Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
    };
    Ok((logger, scanner))
}

fn invoke_pipeline(src: String, logger: FileLogger) {
    if let Ok(mut rt) = Runtime::new() {
        rt.block_on(async move {
            let logger = Arc::new(Mutex::new(logger));
            if let Err(e) = Runner::from_src(src, logger).await.await {
                let _ = term::print_error(&format!("{}", e));
            }
        });
    }
}
