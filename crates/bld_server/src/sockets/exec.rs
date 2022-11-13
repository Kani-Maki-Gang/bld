use crate::extractors::User;
use crate::helpers::enqueue_worker;
use actix::prelude::*;
use actix_web::error::ErrorUnauthorized;
use actix_web::web::{Data, Payload};
use actix_web::{Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::database::pipeline_runs::{
    self, PR_STATE_FAULTED, PR_STATE_FINISHED, PR_STATE_QUEUED,
};
use bld_core::proxies::PipelineFileSystemProxy;
use bld_core::scanner::FileScanner;
use bld_sock::messages::{RunInfo, ServerMessages};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tracing::{debug, error};

pub struct ExecutePipelineSocket {
    config: Data<BldConfig>,
    enqueue_tx: Data<Sender<ServerMessages>>,
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
        enqueue_tx: Data<Sender<ServerMessages>>,
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
            let content = scanner.scan();
            for line in content.iter() {
                ctx.text(line.to_string());
            }
        }
    }

    fn exec(act: &mut Self, ctx: &mut <Self as Actor>::Context) {
        if let Ok(mut conn) = act.pool.get() {
            if let Some(run_id) = act.run_id.as_ref() {
                match pipeline_runs::select_by_id(&mut conn, run_id) {
                    Ok(run) if run.state == PR_STATE_FINISHED || run.state == PR_STATE_FAULTED => {
                        ctx.stop()
                    }
                    Ok(run) if run.state == PR_STATE_QUEUED => {
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

    fn enqueue(&mut self, text: &str) -> Result<()> {
        let data = serde_json::from_str::<RunInfo>(text)?;
        debug!("enqueueing run");
        enqueue_worker(
            &self.user,
            self.proxy.clone(),
            self.pool.clone(),
            self.enqueue_tx.clone(),
            data,
        )
        .map(|run_id| {
            self.scanner = Some(FileScanner::new(Arc::clone(&self.config), &run_id));
            self.run_id = Some(run_id);
        })
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
                if let Err(e) = self.enqueue(&txt) {
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

pub async fn ws_exec(
    user: Option<User>,
    req: HttpRequest,
    stream: Payload,
    cfg: Data<BldConfig>,
    enqueue_tx: Data<Sender<ServerMessages>>,
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
