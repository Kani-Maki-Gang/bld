use crate::extractors::User;
use actix::prelude::*;
use actix_web::{error::ErrorUnauthorized, web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use anyhow::anyhow;
use bld_config::BldConfig;
use bld_core::database::pipeline_runs;
use bld_core::proxies::{PipelineFileSystemProxy, ServerPipelineProxy};
use bld_core::scanner::{FileScanner, Scanner};
use bld_runner::messages::ExecInfo;
use bld_supervisor::base::{UnixSocketWrite, UnixSocketMessage};
use bld_supervisor::client::UnixSocketWriter;
use bld_utils::fs::IsYaml;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::error;
use uuid::Uuid;

pub struct ExecutePipelineSocket {
    hb: Instant,
    config: web::Data<BldConfig>,
    supervisor: web::Data<Mutex<UnixSocketWriter>>,
    pool: web::Data<Pool<ConnectionManager<SqliteConnection>>>,
    proxy: web::Data<ServerPipelineProxy>,
    user: User,
    scanner: Option<FileScanner>,
}

impl ExecutePipelineSocket {
    pub fn new(
        user: User,
        config: web::Data<BldConfig>,
        supervisor: web::Data<Mutex<UnixSocketWriter>>,
        pool: web::Data<Pool<ConnectionManager<SqliteConnection>>>,
        proxy: web::Data<ServerPipelineProxy>,
    ) -> Self {
        Self {
            hb: Instant::now(),
            config,
            supervisor,
            pool,
            proxy,
            user,
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
                ctx.text(line.to_string());
            }
        }
    }

    async fn enqueue_worker(&mut self, data: &str) -> anyhow::Result<()> {
        let info = serde_json::from_str::<ExecInfo>(data)?;
        let path = self.proxy.path(&info.name)?;
        if !path.is_yaml() {
            let message = String::from("pipeline file not found");
            return Err(anyhow!(message));
        }

        let run_id = Uuid::new_v4().to_string();
        let conn = self.pool.get()?;
        pipeline_runs::insert(&conn, &run_id, &info.name, &self.user.name)?;
        let vars = info.variables.map(|hmap| {
            hmap.iter()
                .map(|(k, v)| format!("{k}={v}"))
                .fold(String::new(), |acc, n| format!("{acc} {n}"))
        });
        let env = info.environment.map(|hmap| {
            hmap.iter()
                .map(|(k, v)| format!("{k}={v}"))
                .fold(String::new(), |acc, n| format!("{acc} {n}"))
        });

        let supervisor = self.supervisor.lock().unwrap();
        supervisor.try_write(&UnixSocketMessage::ServerEnqueue { 
            pipeline: info.name.to_string(), 
            run_id: run_id.to_string(), 
            variables: vars, 
            environment: env,
        }).await?;

        self.scanner = Some(FileScanner::new(Arc::clone(&self.config), &run_id));
        Ok(())
    }
}

impl Actor for ExecutePipelineSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(Duration::from_millis(500), |act, ctx| {
            ExecutePipelineSocket::heartbeat(act, ctx);
            ExecutePipelineSocket::scan(act, ctx);
        });
        ctx.run_interval(Duration::from_secs(10), |act, ctx| {
            ExecutePipelineSocket::scan(act, ctx);
        });
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ExecutePipelineSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(txt)) => {
                ctx.wait(async {
                    if let Err(e) = self.enqueue_worker(&txt).await {
                       error!("{}", e.to_string());
                       ctx.text("Unable to run pipeline");
                       ctx.stop();
                    }
                });
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

pub async fn ws_exec(
    user: Option<User>,
    req: HttpRequest,
    stream: web::Payload,
    cfg: web::Data<BldConfig>,
    supervisor: web::Data<Mutex<UnixSocketWriter>>,
    pool: web::Data<Pool<ConnectionManager<SqliteConnection>>>,
    proxy: web::Data<ServerPipelineProxy>,
) -> Result<HttpResponse, Error> {
    let user = user.ok_or_else(|| ErrorUnauthorized(""))?;
    println!("{req:?}");
    let socket = ExecutePipelineSocket::new(user, cfg, supervisor, pool, proxy);
    let res = ws::start(socket, &req, stream);
    println!("{res:?}");
    res
}
