use crate::config::BldConfig;
use crate::persist::{Database, FileScanner, Scanner};
use actix::prelude::*;
use actix_web_actors::ws;
use std::io::{self, Error, ErrorKind};
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub struct MonitorPipelineSocket {
    hb: Instant,
    id: String,
    config: Option<BldConfig>,
    scanner: Option<FileScanner>,
    db: Option<Database>,
}

impl MonitorPipelineSocket {
    pub fn new() -> Self {
        let config = match BldConfig::load() {
            Ok(config) => Some(config),
            Err(_) => None,
        };
        Self {
            hb: Instant::now(),
            id: String::new(),
            config,
            scanner: None,
            db: None,
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
        if let Some(db) = act.db.as_mut() {
            db.load(&act.id);
            match &db.pipeline {
                Some(pipeline) => {
                    if !pipeline.running {
                        ctx.stop();
                    }
                }
                None => {
                    ctx.text("internal server error");
                    ctx.stop();
                }
            }
        }
    }

    fn dependencies(&mut self, id: &str) -> io::Result<()> {
        let config = match &self.config {
            Some(config) => config,
            None => return Err(Error::new(ErrorKind::Other, "config not loaded")),
        };

        let mut db = match Database::connect(&config.local.db) {
            Ok(db) => db,
            Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
        };
        db.load(id);
        let pipeline = match &db.pipeline {
            Some(pipeline) => pipeline,
            None => return Err(Error::new(ErrorKind::Other, "pipeline not found")),
        };

        self.id = id.to_string();

        let path = {
            let mut path = PathBuf::new();
            path.push(&config.local.logs);
            path.push(format!("{}-{}", pipeline.name, pipeline.id));
            path.display().to_string()
        };

        self.scanner = match FileScanner::new(&path) {
            Ok(sc) => Some(sc),
            Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
        };

        self.db = Some(db);

        Ok(())
    }
}

impl Actor for MonitorPipelineSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(Duration::from_secs(1), |act, ctx| {
            MonitorPipelineSocket::heartbeat(act, ctx);
            MonitorPipelineSocket::scan(act, ctx);
        });
        ctx.run_interval(Duration::from_secs(1), |act, ctx| {
            MonitorPipelineSocket::exec(act, ctx);
        });
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MonitorPipelineSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(txt)) => {
                if let Err(e) = self.dependencies(&txt) {
                    eprintln!("{}", e.to_string());
                    ctx.text("internal server error");
                    ctx.stop();
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
