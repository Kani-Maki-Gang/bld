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
use actix_ws::Session;
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::{fs::FileSystem, scanner::FileScanner};
use bld_models::{
    dtos::{ExecClientMessage, ExecServerMessage},
    pipeline_runs::{self, PR_STATE_FAULTED, PR_STATE_FINISHED, PR_STATE_QUEUED},
};
use bld_sock::session::{self, WebSocketMessage};
use sea_orm::DatabaseConnection;
use std::time::Duration;
use tracing::{debug, error};

const SCAN_INTERVAL_MS: u64 = 500;
const STATE_CHECK_INTERVAL_MS: u64 = 1000;

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
            if let Err(e) = session.text(data).await {
                error!("{e}");
            }
        }
    }

    pub async fn check_state(&self, session: &mut Session) -> bool {
        let Some(run_id) = self.run_id.as_ref() else {
            return true;
        };
        match pipeline_runs::select_by_id(self.conn.as_ref(), run_id).await {
            Ok(run) if run.state == PR_STATE_FINISHED || run.state == PR_STATE_FAULTED => {
                debug!("run is in a {} state", run.state);
                false
            }
            Ok(run) if run.state == PR_STATE_QUEUED => {
                debug!("run is in a {} state", PR_STATE_QUEUED);
                let message = format!(
                    "run with id {run_id} has been queued, use the monit command to see the output when it's started"
                );
                if let Err(e) = session.text(message.as_str()).await {
                    error!("{e}");
                }
                false
            }
            Err(e) => {
                debug!("run encountered error {e}");
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
        let username = self.user.name.to_owned();
        let fs = self.fs.clone().into_inner();
        let pool = self.conn.clone().into_inner();
        let supervisor = self.supervisor.clone().into_inner();

        debug!("enqueueing run");
        let run_id = enqueue_worker(&username, fs, pool, supervisor, message).await?;
        self.scanner
            .replace(FileScanner::new(self.config.as_ref(), &run_id));
        self.run_id.replace(run_id.to_owned());
        let message = ExecServerMessage::QueuedRun { run_id };
        if let Ok(data) = serde_json::to_string(&message) {
            session.text(data).await?;
        }
        Ok(())
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
    let (response, mut handler) = session::handle(&req, body)?;

    spawn(async move {
        debug!("spawned exec web socket");
        let mut scan_interval = time::interval(Duration::from_millis(SCAN_INTERVAL_MS));
        let mut state_interval = time::interval(Duration::from_millis(STATE_CHECK_INTERVAL_MS));

        loop {
            tokio::select! {
                msg = handler.next() =>  {
                    match msg {
                        WebSocketMessage::Text(txt) => {
                            let session = handler.session();
                            if let Err(e) = socket.handle_message(session, &txt).await {
                                error!("handling message error. {e}");
                                let _ = session.text(e.to_string()).await.inspect_err(|e| error!("{e}"));
                                handler.error();
                                break;
                            }
                        }
                        WebSocketMessage::Continue => {}
                        _ => break,
                    }
                }

                _ = scan_interval.tick() => {
                    let session = handler.session();
                    socket.scan(session).await;
                }

                _ = state_interval.tick() => {
                    let session = handler.session();
                    if !socket.check_state(session).await {
                        break;
                    }
                }
            }
        }

        handler.cleanup().await;
    });

    Ok(response)
}
