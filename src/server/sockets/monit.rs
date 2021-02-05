use crate::config::BldConfig;
use crate::path;
use crate::persist::{Database, FileScanner, Scanner};
use crate::server::User;
use crate::types::{BldError, Result};
use actix::prelude::*;
use actix_web::{error::ErrorUnauthorized, web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use std::path::PathBuf;
use std::time::{Duration, Instant};

type StdResult<T, V> = std::result::Result<T, V>;

pub struct MonitorPipelineSocket {
    hb: Instant,
    id: String,
    config: web::Data<BldConfig>,
    scanner: Option<FileScanner>,
    db: Option<Database>,
}

impl MonitorPipelineSocket {
    pub fn new(config: web::Data<BldConfig>) -> Self {
        Self {
            hb: Instant::now(),
            id: String::new(),
            config: config.clone(),
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

    fn dependencies(&mut self, id: &str) -> Result<()> {
        let config = self.config.get_ref();
        let mut db = Database::connect(&config.local.db)?;
        db.load(id);
        let pipeline = match &db.pipeline {
            Some(pipeline) => pipeline,
            None => return Err(BldError::Other("pipeline not found".to_string())),
        };

        self.id = id.to_string();

        let path = path![
            &config.local.logs,
            format!("{}-{}", pipeline.name, pipeline.id)
        ]
        .display()
        .to_string();

        self.scanner = Some(FileScanner::new(&path)?);
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

impl StreamHandler<StdResult<ws::Message, ws::ProtocolError>> for MonitorPipelineSocket {
    fn handle(&mut self, msg: StdResult<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
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

pub async fn ws_monit(
    user: Option<User>,
    req: HttpRequest,
    stream: web::Payload,
    config: web::Data<BldConfig>,
) -> StdResult<HttpResponse, Error> {
    if user.is_none() {
        return Err(ErrorUnauthorized(""));
    }

    println!("{:?}", req);
    let res = ws::start(MonitorPipelineSocket::new(config), &req, stream);
    println!("{:?}", res);
    res
}
