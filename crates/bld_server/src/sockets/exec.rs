use crate::{
    extractors::User,
    supervisor::{channel::SupervisorMessageSender, helpers::enqueue_worker},
};
use actix_web::{
    HttpRequest, Responder,
    error::ErrorUnauthorized,
    rt::{spawn, time},
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
use futures::StreamExt;
use sea_orm::DatabaseConnection;
use std::{sync::Arc, time::Duration};
use tracing::{debug, error};

struct ExecWebsocket {
    config: Data<BldConfig>,
    supervisor: Data<SupervisorMessageSender>,
    conn: Data<DatabaseConnection>,
    fs: Data<FileSystem>,
    user: User,
    scanner: Option<FileScanner>,
    run_id: Option<String>,
}

impl ExecWebsocket {
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

    pub async fn scan(&self, session: &mut Session) {
        let Some(scanner) = self.scanner.as_ref() else {
            return;
        };
        let Ok(lines) = scanner.scan().await else {
            return;
        };
        for line in lines.iter() {
            let message = ExecServerMessage::Log {
                content: line.to_string(),
            };
            let Ok(data) = serde_json::to_string(&message) else {
                continue;
            };
            session.text(data);
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
                false
            }
            Err(_) => {
                if let Err(e) = session.text("internal server error").await {
                    error!("{e}");
                }
                false
            }
            _ => true,
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
    let mut socket = ExecWebsocket::new(config, supervisor, conn, fs, user);
    let (response, mut session, mut msg_stream) = handle(&req, body)?;

    spawn(async move {
        let mut reason: Option<CloseReason> = None;
        let mut scan_interval = time::interval(Duration::from_millis(500));
        let mut exec_interval = time::interval(Duration::from_millis(500));

        loop {
            let Some(Ok(msg)) = msg_stream.next().await else {
                break;
            };

            match msg {
                Message::Text(txt) => {
                    if let Err(e) = socket.handle_message(&mut session, &txt).await {
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

            scan_interval.tick().await;
            socket.scan(&mut session).await;

            exec_interval.tick().await;
            if !socket.exec(&mut session).await {
                break;
            }
        }

        if let Err(e) = session.close(reason).await {
            error!("encountered error while closing websocket session due to {e}");
        }
    });

    Ok(response)
}
