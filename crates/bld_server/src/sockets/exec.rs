use crate::{
    extractors::User,
    supervisor::{channel::SupervisorMessageSender, helpers::enqueue_worker},
};
use actix::prelude::*;
use actix_web::{
    error::ErrorUnauthorized,
    web::{Data, Payload},
    Error, HttpRequest, HttpResponse,
};
use actix_web_actors::ws;
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::{
    database::pipeline_runs::{self, PR_STATE_FAULTED, PR_STATE_FINISHED, PR_STATE_QUEUED},
    messages::{ExecClientMessage, ExecServerMessage},
    proxies::PipelineFileSystemProxy,
    scanner::FileScanner,
};
use futures_util::future::ready;
use sea_orm::DatabaseConnection;
use std::{sync::Arc, time::Duration};
use tracing::{debug, error};

pub struct ExecutePipelineSocket {
    config: Data<BldConfig>,
    supervisor: Data<SupervisorMessageSender>,
    conn: Data<DatabaseConnection>,
    proxy: Data<PipelineFileSystemProxy>,
    user: User,
    scanner: Option<FileScanner>,
    run_id: Option<String>,
}

impl ExecutePipelineSocket {
    pub fn new(
        user: User,
        config: Data<BldConfig>,
        supervisor_sender: Data<SupervisorMessageSender>,
        conn: Data<DatabaseConnection>,
        proxy: Data<PipelineFileSystemProxy>,
    ) -> Self {
        Self {
            config,
            supervisor: supervisor_sender,
            conn,
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
                let message = ExecServerMessage::Log {
                    content: line.to_string(),
                };
                let _ = serde_json::to_string(&message).map(|data| ctx.text(data));
            }
        }
    }

    fn exec(act: &mut Self, ctx: &mut <Self as Actor>::Context) {
        if let Some(run_id) = act.run_id.as_ref() {
            async move { pipeline_runs::select_by_id(act.conn.as_ref(), run_id).await }
                .into_actor(act)
                .then(|res, act, ctx| match res {
                    Ok(run) if run.state == PR_STATE_FINISHED || run.state == PR_STATE_FAULTED => {
                        ctx.stop();
                        ready(())
                    }
                    Ok(run) if run.state == PR_STATE_QUEUED => {
                        ctx.text("run with id {run_id} has been queued, use the monit command to see the output when it's started");
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
    }

    fn handle_client_message(
        &mut self,
        message: &str,
        ctx: &mut <Self as Actor>::Context,
    ) -> Result<()> {
        let message: ExecClientMessage = serde_json::from_str(message)?;

        debug!("enqueueing run");

        let username = self.user.name.to_owned();
        let proxy = Arc::clone(&self.proxy);
        let pool = Arc::clone(&self.conn);
        let supervisor = Arc::clone(&self.supervisor);

        let enqueue_fut =
            async move { enqueue_worker(&username, proxy, pool, supervisor, message).await }
                .into_actor(self)
                .then(|res, act, ctx| match res {
                    Ok(run_id) => {
                        act.scanner = Some(FileScanner::new(Arc::clone(&act.config), &run_id));
                        act.run_id = Some(run_id.to_owned());
                        let message = ExecServerMessage::QueuedRun { run_id };
                        if let Ok(data) = serde_json::to_string(&message) {
                            ctx.text(data);
                        }
                        ready(())
                    }
                    Err(e) => {
                        error!("{e}");
                        ctx.text(e.to_string());
                        ctx.stop();
                        ready(())
                    }
                });

        ctx.spawn(enqueue_fut);

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
                if let Err(e) = self.handle_client_message(&txt, ctx) {
                    ctx.text(e.to_string());
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
    cfg: Data<BldConfig>,
    supervisor_sender: Data<SupervisorMessageSender>,
    conn: Data<DatabaseConnection>,
    proxy: Data<PipelineFileSystemProxy>,
) -> Result<HttpResponse, Error> {
    let user = user.ok_or_else(|| ErrorUnauthorized(""))?;
    println!("{req:?}");
    let socket = ExecutePipelineSocket::new(user, cfg, supervisor_sender, conn, proxy);
    let res = ws::start(socket, &req, stream);
    println!("{res:?}");
    res
}
