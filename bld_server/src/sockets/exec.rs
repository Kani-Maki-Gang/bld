use crate::data::PipelineWorker;
use crate::extractors::User;
use actix::prelude::*;
use actix_web::{error::ErrorUnauthorized, web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use anyhow::anyhow;
use bld_config::{path, BldConfig};
use bld_core::database::pipeline_runs;
use bld_core::proxies::{PipelineFileSystemProxy, ServerPipelineProxy};
use bld_core::scanner::{FileScanner, Scanner};
use bld_runner::messages::ExecInfo;
use bld_runner::{Pipeline, Runner, RunnerBuilder};
use bld_utils::fs::IsYaml;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use tracing::error;
use uuid::Uuid;

pub struct ExecutePipelineSocket {
    hb: Instant,
    config: web::Data<BldConfig>,
    pool: web::Data<Pool<ConnectionManager<SqliteConnection>>>,
    proxy: web::Data<ServerPipelineProxy>,
    workers: web::Data<Mutex<Vec<PipelineWorker>>>,
    user: User,
    worker_idx: Option<usize>,
    scanner: Option<FileScanner>,
}

impl ExecutePipelineSocket {
    pub fn new(
        user: User,
        config: web::Data<BldConfig>,
        pool: web::Data<Pool<ConnectionManager<SqliteConnection>>>,
        proxy: web::Data<ServerPipelineProxy>,
        workers: web::Data<Mutex<Vec<PipelineWorker>>>,
    ) -> Self {
        Self {
            hb: Instant::now(),
            config,
            pool,
            proxy,
            workers,
            user,
            worker_idx: None,
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

    fn worker(act: &mut Self, ctx: &mut <Self as Actor>::Context) {
        let idx = match act.worker_idx {
            Some(idx) => idx,
            None => return,
        };
        let mut workers = act.workers.lock().unwrap();
        if let Some(worker) = workers.iter_mut().nth(idx).as_mut() {
            if worker.completed() {
                ctx.stop();
            }
        }
    }

    fn spawn_worker(&mut self, data: &str) -> anyhow::Result<()> {
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
        let mut cmd = Command::new(std::env::current_exe()?);
        cmd.arg("worker");
        cmd.arg("--pipeline");
        cmd.arg(&info.name);
        cmd.arg("--run-id");
        cmd.arg(&run_id);
        if let Some(vars) = vars {
            cmd.arg("--variables");
            cmd.arg(&vars);
        }
        if let Some(env) = env {
            cmd.arg("--environment");
            cmd.arg(&env);
        }

        let mut worker = PipelineWorker::new(cmd);
        let res = worker.spawn();
        let mut workers = self.workers.lock().unwrap();
        workers.push(worker);

        self.worker_idx = Some(workers.len() - 1);
        self.scanner = Some(FileScanner::new(Arc::clone(&self.config), &run_id));
        res
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
            ExecutePipelineSocket::worker(act, ctx);
        });
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ExecutePipelineSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(txt)) => {
                if let Err(e) = self.spawn_worker(&txt) {
                    error!("{}", e.to_string());
                    ctx.text("Unable to run pipeline");
                    ctx.stop();
                };
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
    pool: web::Data<Pool<ConnectionManager<SqliteConnection>>>,
    proxy: web::Data<ServerPipelineProxy>,
    workers: web::Data<Mutex<Vec<PipelineWorker>>>,
) -> Result<HttpResponse, Error> {
    let user = user.ok_or_else(|| ErrorUnauthorized(""))?;
    println!("{req:?}");
    let socket = ExecutePipelineSocket::new(user, cfg, pool, proxy, workers);
    let res = ws::start(socket, &req, stream);
    println!("{res:?}");
    res
}
