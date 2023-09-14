use crate::extractors::User;
use actix::prelude::*;
use actix_web::{
    error::ErrorUnauthorized,
    web::{Data, Payload},
    Error, HttpRequest, HttpResponse,
};
use actix_web_actors::ws;
use anyhow::{anyhow, bail, Result};
use bld_config::BldConfig;
use bld_core::{
    database::pipeline_runs::{self, PR_STATE_FAULTED, PR_STATE_FINISHED},
    messages::MonitInfo,
    scanner::FileScanner,
};
use futures_util::future::ready;
use sea_orm::DatabaseConnection;
use std::{sync::Arc, time::Duration};

pub struct MonitorPipelineSocket {
    id: String,
    conn: Data<DatabaseConnection>,
    config: Data<BldConfig>,
    scanner: Option<FileScanner>,
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
        if let Some(scanner) = act.scanner.as_mut() {
            let content = scanner.scan();
            for line in content.iter() {
                ctx.text(line.to_string());
            }
        }
    }

    fn exec(act: &mut Self) {
        let conn = act.conn.clone();
        let id = act.id.to_owned();
        let _ = async move { pipeline_runs::select_by_id(conn.as_ref(), &id).await }
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
    }

    fn dependencies(&mut self, data: &str) {
        let conn = self.conn.clone();
        let _ = async move {
            let data = serde_json::from_str::<MonitInfo>(data)?;
            let run = if data.last {
                pipeline_runs::select_last(conn.as_ref()).await
            } else if let Some(id) = data.id {
                pipeline_runs::select_by_id(conn.as_ref(), &id).await
            } else if let Some(name) = data.name {
                pipeline_runs::select_by_name(conn.as_ref(), &name).await
            } else {
                bail!("pipeline not found");
            };
            run.map_err(|_| anyhow!("pipeline not found"))
        }
        .into_actor(self)
        .then(|res, _, ctx| match res {
            Ok(run) => {
                self.id = run.id.clone();
                self.scanner = Some(FileScanner::new(Arc::clone(&self.config), &run.id));
                ready(())
            }
            Err(e) => {
                eprintln!("{e}");
                ctx.text("internal server error");
                ctx.stop();
                ready(())
            }
        });
    }
}

impl Actor for MonitorPipelineSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(Duration::from_millis(500), |act, ctx| {
            MonitorPipelineSocket::scan(act, ctx);
        });
        ctx.run_interval(Duration::from_secs(1), |act, _| {
            MonitorPipelineSocket::exec(act);
        });
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MonitorPipelineSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(txt)) => {
                self.dependencies(&txt);
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
