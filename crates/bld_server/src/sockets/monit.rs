use crate::extractors::User;
use actix::prelude::*;
use actix_web::{
    error::ErrorUnauthorized,
    web::{Data, Payload},
    Error, HttpRequest, HttpResponse,
};
use actix_web_actors::ws;
use anyhow::{anyhow, Result, bail};
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

    fn exec(act: &mut Self, ctx: &mut <Self as Actor>::Context) {
        async move { pipeline_runs::select_by_id(act.conn.as_ref(), &act.id).await }
            .into_actor(act)
            .then(|res, act, ctx| match res {
                Ok(run) if run.state == PR_STATE_FINISHED || run.state == PR_STATE_FAULTED => {
                    ctx.stop();
                    ready(())
                }
                Err(_) => {
                    ctx.text("internal server error");
                    ctx.stop();
                    ready(())
                }
                _ => ready(())
            });
    }

    fn dependencies(&mut self, data: &str) -> Result<()> {
        let data = serde_json::from_str::<MonitInfo>(data)?;
        let mut conn = self.conn.get()?;

        let run = if data.last {
            pipeline_runs::select_last(&mut conn)
        } else if let Some(id) = data.id {
            pipeline_runs::select_by_id(&mut conn, &id)
        } else if let Some(name) = data.name {
            pipeline_runs::select_by_name(&mut conn, &name)
        } else {
            bail!("pipeline not found");
        }
        .map_err(|_| anyhow!("pipeline not found"))?;

        self.id = run.id.clone();

        self.scanner = Some(FileScanner::new(Arc::clone(&self.config), &run.id));
        Ok(())
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
                if let Err(e) = self.dependencies(&txt) {
                    eprintln!("{e}");
                    ctx.text("internal server error");
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

pub async fn ws(
    user: Option<User>,
    req: HttpRequest,
    stream: Payload,
    pool: Data<Pool<ConnectionManager<DbConnection>>>,
    config: Data<BldConfig>,
) -> Result<HttpResponse, Error> {
    if user.is_none() {
        return Err(ErrorUnauthorized(""));
    }
    println!("{req:?}");
    let res = ws::start(MonitorPipelineSocket::new(pool, config), &req, stream);
    println!("{res:?}");
    res
}
