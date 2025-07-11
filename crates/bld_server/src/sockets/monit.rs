use crate::extractors::User;
use actix::prelude::*;
use actix_web::{
    Error, HttpRequest, HttpResponse,
    error::ErrorUnauthorized,
    web::{Data, Payload},
};
use actix_web_actors::ws;
use anyhow::{Result, anyhow, bail};
use bld_config::BldConfig;
use bld_core::scanner::FileScanner;
use bld_models::{
    dtos::MonitInfo,
    pipeline_runs::{self, PR_STATE_FAULTED, PR_STATE_FINISHED},
};
use bld_utils::sync::IntoArc;
use futures_util::future::ready;
use sea_orm::DatabaseConnection;
use std::{sync::Arc, time::Duration};
use tracing::{debug, error};

pub struct MonitorPipelineSocket {
    id: String,
    conn: Data<DatabaseConnection>,
    config: Data<BldConfig>,
    scanner: Option<Arc<FileScanner>>,
}

impl MonitorPipelineSocket {
    pub fn new(conn: Data<DatabaseConnection>, config: Data<BldConfig>) -> Self {
        Self {
            id: String::new(),
            conn,
            config,
            scanner: None,
        }
    }

    fn scan(act: &mut Self, ctx: &mut <Self as Actor>::Context) {
        let Some(scanner) = act.scanner.as_ref() else {
            return;
        };
        let scanner = scanner.clone();
        let scan_fut = async move { scanner.scan().await }
            .into_actor(act)
            .then(|res, _, ctx| {
                if let Ok(lines) = res {
                    for line in lines.iter() {
                        ctx.text(line.to_string());
                    }
                }
                ready(())
            });
        ctx.spawn(scan_fut);
    }

    fn exec(act: &mut Self, ctx: &mut <Self as Actor>::Context) {
        let conn = act.conn.clone();
        let id = act.id.to_owned();
        let select_fut = async move { pipeline_runs::select_by_id(conn.as_ref(), &id).await }
            .into_actor(act)
            .then(|res, _, ctx| match res {
                Ok(run) if run.state == PR_STATE_FINISHED || run.state == PR_STATE_FAULTED => {
                    ctx.stop();
                    ready(())
                }
                Err(_) => {
                    ctx.text("internal server error");
                    ctx.stop();
                    ready(())
                }
                _ => ready(()),
            });
        ctx.spawn(select_fut);
    }

    fn dependencies(&mut self, data: String, ctx: &mut <Self as Actor>::Context) {
        let conn = self.conn.clone();
        let pipeline_fut = async move {
            let data = serde_json::from_str::<MonitInfo>(&data)?;
            let run = if data.last {
                pipeline_runs::select_last(conn.as_ref()).await
            } else if let Some(id) = data.id {
                pipeline_runs::select_by_id(conn.as_ref(), &id).await
            } else if let Some(name) = data.name {
                pipeline_runs::select_by_name(conn.as_ref(), &name).await
            } else {
                bail!("file not found");
            };
            run.map_err(|_| anyhow!("file not found"))
        }
        .into_actor(self)
        .then(|res, act, ctx| match res {
            Ok(run) => {
                debug!("starting scan for run");
                act.id.clone_from(&run.id);
                act.scanner = Some(FileScanner::new(act.config.as_ref(), &run.id).into_arc());
                ready(())
            }
            Err(e) => {
                error!("{e}");
                ctx.text("internal server error");
                ctx.stop();
                ready(())
            }
        });

        ctx.spawn(pipeline_fut);
    }
}

impl Actor for MonitorPipelineSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(Duration::from_millis(500), |act, ctx| {
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
                debug!("received message {txt}");
                self.dependencies(txt.to_string(), ctx);
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

pub async fn ws(
    user: Option<User>,
    req: HttpRequest,
    stream: Payload,
    conn: Data<DatabaseConnection>,
    config: Data<BldConfig>,
) -> Result<HttpResponse, Error> {
    if user.is_none() {
        return Err(ErrorUnauthorized(""));
    }
    println!("{req:?}");
    let res = ws::start(MonitorPipelineSocket::new(conn, config), &req, stream);
    println!("{res:?}");
    res
}
