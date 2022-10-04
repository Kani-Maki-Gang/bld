use crate::extractors::User;
use actix::prelude::*;
use actix_web::{
    error::ErrorUnauthorized,
    rt::spawn,
    web::{Data, Payload},
    Error, HttpRequest, HttpResponse,
};
use actix_web_actors::ws;
use anyhow::anyhow;
use bld_config::BldConfig;
use bld_core::database::pipeline_runs;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_core::scanner::{FileScanner, Scanner};
use bld_runner::messages::ExecInfo;
use bld_supervisor::base::ServerMessages;
use bld_utils::fs::IsYaml;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tracing::{debug, error};
use uuid::Uuid;

pub struct ExecutePipelineSocket {
    config: Data<BldConfig>,
    enqueue_tx: Data<Mutex<Sender<ServerMessages>>>,
    pool: Data<Pool<ConnectionManager<SqliteConnection>>>,
    proxy: Data<PipelineFileSystemProxy>,
    user: User,
    scanner: Option<FileScanner>,
    run_id: Option<String>,
}

impl ExecutePipelineSocket {
    pub fn new(
        user: User,
        config: Data<BldConfig>,
        enqueue_tx: Data<Mutex<Sender<ServerMessages>>>,
        pool: Data<Pool<ConnectionManager<SqliteConnection>>>,
        proxy: Data<PipelineFileSystemProxy>,
    ) -> Self {
        Self {
            config,
            enqueue_tx,
            pool,
            proxy,
            user,
            scanner: None,
            run_id: None,
        }
    }

    fn scan(act: &mut Self, ctx: &mut <Self as Actor>::Context) {
        if let Some(scanner) = act.scanner.as_mut() {
            let content = scanner.fetch();
            for line in content.iter() {
                ctx.text(line.to_string());
            }
        }
    }

    fn exec(act: &mut Self, ctx: &mut <Self as Actor>::Context) {
        if let Ok(connection) = act.pool.get() {
            if let Some(run_id) = act.run_id.as_ref() {
                match pipeline_runs::select_by_id(&connection, run_id) {
                    Ok(run) if run.state == "finished" => ctx.stop(),
                    Ok(run) if run.state == "queued" => {
                        ctx.text("run with id {run_id} has been queued, use the monit command to see the output when it's started");
                        ctx.stop()
                    }
                    Err(_) => {
                        ctx.text("internal server error");
                        ctx.stop();
                    }
                    _ => {}
                }
            }
        }
    }

    fn enqueue_worker(&mut self, data: &str) -> anyhow::Result<()> {
        let info = serde_json::from_str::<ExecInfo>(data)?;
        let path = self.proxy.path(&info.name)?;
        if !path.is_yaml() {
            let message = String::from("pipeline file not found");
            return Err(anyhow!(message));
        }

        let run_id = Uuid::new_v4().to_string();
        let conn = self.pool.get()?;
        pipeline_runs::insert(&conn, &run_id, &info.name, &self.user.name)?;
        let variables = info.variables.map(hash_map_to_var_string);
        let environment = info.environment.map(hash_map_to_var_string);

        self.scanner = Some(FileScanner::new(Arc::clone(&self.config), &run_id));
        self.run_id = Some(run_id.clone());

        let enqueue_tx = self.enqueue_tx.clone();
        spawn(async move {
            let tx = enqueue_tx.lock().await;
            let msg = ServerMessages::Enqueue {
                pipeline: info.name.to_string(),
                run_id,
                variables,
                environment,
            };
            match tx.send(msg).await {
                Ok(_) => debug!("sent message to supervisor receiver"),
                Err(e) => error!("unable to send message to supervisor receiver. {e}"),
            }
        });
        Ok(())
    }
}

impl Actor for ExecutePipelineSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(Duration::from_millis(500), |act, ctx| {
            ExecutePipelineSocket::scan(act, ctx);
        });
        ctx.run_interval(Duration::from_secs(1), |act, ctx| {
            ExecutePipelineSocket::exec(act, ctx);
        });
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ExecutePipelineSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(txt)) => {
                if let Err(e) = self.enqueue_worker(&txt) {
                    error!("{}", e.to_string());
                    ctx.text("Unable to run pipeline");
                    ctx.stop();
                }
            }
            Ok(ws::Message::Ping(msg)) => {
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {}
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

fn hash_map_to_var_string(hmap: HashMap<String, String>) -> String {
    hmap.iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<String>>()
        .join(" ")
}

pub async fn ws_exec(
    user: Option<User>,
    req: HttpRequest,
    stream: Payload,
    cfg: Data<BldConfig>,
    enqueue_tx: Data<Mutex<Sender<ServerMessages>>>,
    pool: Data<Pool<ConnectionManager<SqliteConnection>>>,
    proxy: Data<PipelineFileSystemProxy>,
) -> Result<HttpResponse, Error> {
    let user = user.ok_or_else(|| ErrorUnauthorized(""))?;
    println!("{req:?}");
    let socket = ExecutePipelineSocket::new(user, cfg, enqueue_tx, pool, proxy);
    let res = ws::start(socket, &req, stream);
    println!("{res:?}");
    res
}
