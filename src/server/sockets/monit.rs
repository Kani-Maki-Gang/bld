use crate::config::BldConfig;
use crate::monit::MonitInfo;
use crate::path;
use crate::persist::{ConnectionPool, FileScanner, Scanner, PipelineModel};
use crate::server::User;
use actix::prelude::*;
use actix_web::{error::ErrorUnauthorized, web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use anyhow::anyhow;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub struct MonitorPipelineSocket {
    hb: Instant,
    id: String,
    db_pool: web::Data<ConnectionPool>,
    config: web::Data<BldConfig>,
    scanner: Option<FileScanner>,
}

impl MonitorPipelineSocket {
    pub fn new(db_pool: web::Data<ConnectionPool>, config: web::Data<BldConfig>) -> Self {
        Self {
            hb: Instant::now(),
            id: String::new(),
            db_pool: db_pool.clone(),
            config,
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
        if let Ok(connection) = act.db_pool.get() {
            match PipelineModel::select_by_id(&connection, &act.id) {
                Ok(PipelineModel { running: false, .. }) => ctx.stop(),
                Err(_) => {
                    ctx.text("internal server error");
                    ctx.stop();
                },
                _ => {}
            }
        }
    }

    fn dependencies(&mut self, data: &str) -> anyhow::Result<()> {
        let data = serde_json::from_str::<MonitInfo>(data)?;
        let config = self.config.get_ref();
        let connection = self.db_pool.get()?;

        let pipeline = if data.last {
            PipelineModel::select_last(&connection)
        } else if let Some(id) = data.id {
            PipelineModel::select_by_id(&connection, &id)
        } else if let Some(name) = data.name {
            PipelineModel::select_by_name(&connection, &name)
        } else {
            return Err(anyhow!("pipeline not found"));
        }
        .map_err(|_| anyhow!("pipeline not found"))?;

        self.id = pipeline.id.clone();

        let path = path![
            &config.local.logs,
            format!("{}-{}", pipeline.name, pipeline.id)
        ]
        .display()
        .to_string();

        self.scanner = Some(FileScanner::new(&path)?);
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

pub async fn ws_monit(
    user: Option<User>,
    req: HttpRequest,
    stream: web::Payload,
    db_pool: web::Data<ConnectionPool>,
    config: web::Data<BldConfig>,
) -> Result<HttpResponse, Error> {
    if user.is_none() {
        return Err(ErrorUnauthorized(""));
    }
    println!("{:?}", req);
    let res = ws::start(MonitorPipelineSocket::new(db_pool, config), &req, stream);
    println!("{:?}", res);
    res
}
