use crate::config::BldConfig;
use crate::persist::{Database, FileLogger, FileScanner, Scanner};
use crate::run::Runner;
use crate::term;
use actix::prelude::*;
use actix_web_actors::ws;
use serde_json::Value;
use std::io::{self, Error, ErrorKind};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use uuid::Uuid;

pub struct ExecutePipelineSocket {
    hb: Instant,
    config: Option<BldConfig>,
    src: Option<String>,
    exec: Option<Arc<Mutex<Database>>>,
    logger: Option<Arc<Mutex<FileLogger>>>,
    scanner: Option<FileScanner>,
}

impl ExecutePipelineSocket {
    pub fn new() -> Self {
        let config = match BldConfig::load() {
            Ok(config) => Some(config),
            Err(_) => None,
        };
        Self {
            hb: Instant::now(),
            config,
            src: None,
            exec: None,
            logger: None,
            scanner: None,
        }
    }

    fn heartbeat(act: &Self, ctx: &mut <Self as Actor>::Context) {
        if Instant::now().duration_since(act.hb) > Duration::from_secs(10) {
            println!("Websocket heartbeat failed, disconnecting!");
            ctx.stop();
            return;
        }
        ctx.ping(b"");
    }

    fn scan(act: &mut Self, ctx: &mut <Self as Actor>::Context) {
        if let Some(scanner) = act.scanner.as_mut() {
            let content = scanner.fetch();
            for line in content.iter() {
                ctx.text(line);
            }
        }
    }

    fn exec(act: &mut Self, ctx: &mut <Self as Actor>::Context) {
        if let Some(exec) = act.exec.as_mut() {
            let exec = exec.lock().unwrap();
            if let Some(pipeline) = &exec.pipeline {
                if !pipeline.running {
                    ctx.stop();
                }
            }
        }
    }

    fn dependencies(&mut self, text: &str) -> io::Result<()> {
        let id = Uuid::new_v4().to_string();

        let message = match serde_json::from_str::<Value>(text) {
            Ok(message) => message,
            Err(_) => return Err(Error::new(ErrorKind::Other, "message could not be parsed")),
        };

        let name = match message["name"].as_str() {
            Some(n) => n,
            None => return Err(Error::new(ErrorKind::Other, "name not found in message")),
        };

        self.src = match message["pipeline"].as_str() {
            Some(src) => Some(src.to_string()),
            None => {
                return Err(Error::new(
                    ErrorKind::Other,
                    "pipeline not found in message",
                ))
            }
        };

        let config = match &self.config {
            Some(config) => config,
            None => return Err(Error::new(ErrorKind::Other, "config not loaded")),
        };

        let path = {
            let mut path = std::path::PathBuf::new();
            path.push(&config.local.logs);
            path.push(format!("{}-{}", name, id));
            path.display().to_string()
        };

        self.exec = match Database::connect(&config.local.db) {
            Ok(mut db) => {
                let _ = db.add(&id, &name);
                Some(Arc::new(Mutex::new(db)))
            }
            Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
        };

        self.logger = match FileLogger::new(&path) {
            Ok(lg) => Some(Arc::new(Mutex::new(lg))),
            Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
        };

        self.scanner = match FileScanner::new(&path) {
            Ok(sc) => Some(sc),
            Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
        };

        Ok(())
    }
}

impl Actor for ExecutePipelineSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(Duration::from_secs(1), |act, ctx| {
            ExecutePipelineSocket::heartbeat(act, ctx);
            ExecutePipelineSocket::scan(act, ctx);
        });
        ctx.run_interval(Duration::from_secs(5), |act, ctx| {
            ExecutePipelineSocket::exec(act, ctx);
        });
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ExecutePipelineSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(txt)) => {
                if let Err(e) = self.dependencies(&txt) {
                    eprintln!("{}", e.to_string());
                    ctx.text("internal server error");
                    ctx.stop();
                }
                let src = match &self.src {
                    Some(src) => src.clone(),
                    None => {
                        ctx.stop();
                        return;
                    }
                };
                let exec = match &self.exec {
                    Some(exec) => exec.clone(),
                    None => {
                        ctx.stop();
                        return;
                    }
                };
                let lg = match &self.logger {
                    Some(lg) => lg.clone(),
                    None => {
                        ctx.stop();
                        return;
                    }
                };
                std::thread::spawn(move || invoke_pipeline(src, exec, lg));
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

fn invoke_pipeline(src: String, ex: Arc<Mutex<Database>>, lg: Arc<Mutex<FileLogger>>) {
    if let Ok(mut rt) = Runtime::new() {
        rt.block_on(async move {
            let fut = Runner::from_src(src, ex, lg);
            if let Err(e) = fut.await.await {
                let _ = term::print_error(&format!("{}", e));
            }
        });
    }
}
