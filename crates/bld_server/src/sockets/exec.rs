use crate::{
    extractors::User,
    supervisor::{channel::SupervisorMessageSender, helpers::enqueue_worker},
};
use actix::prelude::*;
use actix_web::{
    HttpRequest, Responder,
    error::ErrorUnauthorized,
    rt::spawn,
    web::{Data, Payload},
};
use actix_ws::{CloseReason, Message, Session, handle};
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::{fs::FileSystem, scanner::FileScanner};
use bld_models::{
    dtos::{ExecClientMessage, ExecServerMessage},
    pipeline_runs::{self, PR_STATE_FAULTED, PR_STATE_FINISHED, PR_STATE_QUEUED},
};
use bld_utils::sync::IntoArc;
use futures_util::future::ready;
use sea_orm::DatabaseConnection;
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::{Mutex, RwLock},
    task::JoinHandle,
    time,
};
use tracing::{debug, error};

pub struct ExecutePipelineSocket {
    config: Data<BldConfig>,
    supervisor: Data<SupervisorMessageSender>,
    conn: Data<DatabaseConnection>,
    fs: Data<FileSystem>,
    user: User,
    scanner: Option<Arc<FileScanner>>,
    run_id: Option<String>,
}

impl ExecutePipelineSocket {
    pub fn new(
        user: User,
        config: Data<BldConfig>,
        supervisor_sender: Data<SupervisorMessageSender>,
        conn: Data<DatabaseConnection>,
        fs: Data<FileSystem>,
    ) -> Self {
        Self {
            config,
            supervisor: supervisor_sender,
            conn,
            fs,
            user,
            scanner: None,
            run_id: None,
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
                        let message = ExecServerMessage::Log {
                            content: line.to_string(),
                        };
                        let _ = serde_json::to_string(&message).map(|data| ctx.text(data));
                    }
                }
                ready(())
            });
        ctx.spawn(scan_fut);
    }

    fn exec(act: &mut Self, ctx: &mut <Self as Actor>::Context) {
        if let Some(run_id) = act.run_id.as_ref() {
            let conn = act.conn.clone();
            let run_id = run_id.to_owned();
            let select_fut = async move { pipeline_runs::select_by_id(conn.as_ref(), &run_id).await }
                .into_actor(act)
                .then(|res, _, ctx| match res {
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
            ctx.spawn(select_fut);
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
        let fs = Arc::clone(&self.fs);
        let pool = Arc::clone(&self.conn);
        let supervisor = Arc::clone(&self.supervisor);

        let enqueue_fut =
            async move { enqueue_worker(&username, fs, pool, supervisor, message).await }
                .into_actor(self)
                .then(|res, act, ctx| match res {
                    Ok(run_id) => {
                        act.scanner =
                            Some(FileScanner::new(act.config.as_ref(), &run_id).into_arc());
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

struct ExecState {
    config: Data<BldConfig>,
    supervisor: Data<SupervisorMessageSender>,
    conn: Data<DatabaseConnection>,
    fs: Data<FileSystem>,
    user: User,
    scanner: Option<FileScanner>,
    run_id: Option<String>,
}

impl ExecState {
    pub fn new(
        config: Data<BldConfig>,
        supervisor: Data<SupervisorMessageSender>,
        conn: Data<DatabaseConnection>,
        fs: Data<FileSystem>,
        user: User,
    ) -> Self {
        Self {
            config,
            supervisor,
            conn,
            fs,
            user,
            scanner: None,
            run_id: None,
        }
    }

    pub async fn exec(&self, session: &mut Session) -> bool {
        let Some(run_id) = self.run_id.as_ref() else {
            return false;
        };
        match pipeline_runs::select_by_id(self.conn.as_ref(), &run_id).await {
            Ok(run) if run.state == PR_STATE_FINISHED || run.state == PR_STATE_FAULTED => true,
            Ok(run) if run.state == PR_STATE_QUEUED => {
                let message = format!(
                    "run with id {run_id} has been queued, use the monit command to see the output when it's started"
                );
                if let Err(e) = session.text(message.as_str()).await {
                    error!("{e}");
                }
                true
            }
            Err(_) => {
                if let Err(e) = session.text("internal server error").await {
                    error!("{e}");
                }
                true
            }
            _ => false,
        }
    }

    pub async fn handle_message(&mut self, session: &mut Session, message: &str) -> Result<()> {
        let message: ExecClientMessage = serde_json::from_str(message)?;

        debug!("enqueueing run");

        let username = self.user.name.to_owned();
        let fs = Arc::clone(&self.fs);
        let pool = Arc::clone(&self.conn);
        let supervisor = Arc::clone(&self.supervisor);

        match enqueue_worker(&username, fs, pool, supervisor, message).await {
            Ok(run_id) => {
                self.scanner
                    .replace(FileScanner::new(self.config.as_ref(), &run_id));
                self.run_id.replace(run_id.to_owned());
                let message = ExecServerMessage::QueuedRun { run_id };
                if let Ok(data) = serde_json::to_string(&message) {
                    session.text(data).await;
                }
                Ok(())
            }
            Err(e) => {
                session.text(e.to_string()).await;
                Err(e)
            }
        }
    }
}

pub async fn ws(
    user: Option<User>,
    req: HttpRequest,
    body: Payload,
    config: Data<BldConfig>,
    supervisor: Data<SupervisorMessageSender>,
    conn: Data<DatabaseConnection>,
    fs: Data<FileSystem>,
) -> actix_web::Result<impl Responder> {
    let user = user.ok_or_else(|| ErrorUnauthorized(""))?;
    let mut state = ExecState::new(config, supervisor, conn, fs, user);
    let (response, mut session, mut msg_stream) = handle(&req, body)?;

    spawn(async move {
        let mut reason: Option<CloseReason> = None;

        while let Some(Ok(msg)) = msg_stream.recv().await {
            match msg {
                Message::Text(txt) => {
                    if let Err(e) = state.handle_message(&mut session, &txt).await {
                        error!("handling message error. {e}");
                        break;
                    }
                }
                Message::Ping(msg) => {
                    if let Err(e) = session.pong(&msg).await {
                        error!("{e}");
                        break;
                    }
                }
                Message::Pong(_) | Message::Binary(_) => {}
                Message::Close(r) => {
                    reason = r;
                    break;
                }
                _ => {}
            }
        }

        if let Err(e) = session.close(reason).await {
            error!("encountered error while closing websocket session due to {e}");
        }
    });

    Ok(response)
}
