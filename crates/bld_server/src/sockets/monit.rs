use crate::extractors::User;
use actix_web::{
    HttpRequest, Responder,
    error::ErrorUnauthorized,
    rt::{spawn, time},
    web::{Data, Payload},
};
use actix_ws::Session;
use anyhow::{Result, bail};
use bld_config::BldConfig;
use bld_core::scanner::FileScanner;
use bld_models::{
    dtos::MonitInfo,
    pipeline_runs::{self, PR_STATE_FAULTED, PR_STATE_FINISHED},
};
use bld_sock::session::{self, WebSocketMessage};
use sea_orm::DatabaseConnection;
use std::time::Duration;
use tracing::{debug, error};

const STATE_CHECK_INTERVAL_MS: u64 = 500;

pub struct MonitorPipelineSocket {
    id: Option<String>,
    conn: Data<DatabaseConnection>,
    config: Data<BldConfig>,
    scanner: Option<FileScanner>,
}

impl MonitorPipelineSocket {
    pub fn new(conn: Data<DatabaseConnection>, config: Data<BldConfig>) -> Self {
        Self {
            id: None,
            conn,
            config,
            scanner: None,
        }
    }

    async fn scan(&self, session: &mut Session) {
        let Some(scanner) = self.scanner.as_ref() else {
            return;
        };
        if let Ok(lines) = scanner.scan().await {
            for line in lines.iter() {
                if let Err(e) = session.text(line.to_string()).await {
                    error!("{e}");
                }
            }
        }
    }

    async fn check_state(&self, session: &mut Session) -> bool {
        let Some(run_id) = self.id.as_ref() else {
            return true;
        };
        debug!("checking run state for pipeline run with id {run_id}");
        match pipeline_runs::select_by_id(self.conn.as_ref(), &run_id).await {
            Ok(run) if run.state == PR_STATE_FINISHED || run.state == PR_STATE_FAULTED => false,
            Err(_) => {
                if let Err(e) = session.text("internal server error").await {
                    error!("{e}");
                }
                false
            }
            _ => true,
        }
    }

    async fn dependencies(&mut self, data: &str) -> Result<()> {
        let conn = self.conn.clone();
        let data = serde_json::from_str::<MonitInfo>(data)?;
        debug!("{data:?}");
        let run = if data.last {
            debug!("fetching the last pipeline run");
            pipeline_runs::select_last(conn.as_ref()).await
        } else if let Some(id) = data.id {
            debug!("fetching pipeline run by id {id}");
            pipeline_runs::select_by_id(conn.as_ref(), &id).await
        } else if let Some(name) = data.name {
            debug!("fetching pipeline run by name {name}");
            pipeline_runs::select_by_name(conn.as_ref(), &name).await
        } else {
            bail!("file not found");
        }?;
        debug!("starting scan for run with id {}", run.id);
        self.scanner = Some(FileScanner::new(self.config.as_ref(), &run.id));
        self.id.replace(run.id);
        Ok(())
    }
}

pub async fn ws(
    user: Option<User>,
    req: HttpRequest,
    body: Payload,
    conn: Data<DatabaseConnection>,
    config: Data<BldConfig>,
) -> actix_web::Result<impl Responder> {
    user.ok_or_else(|| ErrorUnauthorized(""))?;
    let mut socket = MonitorPipelineSocket::new(conn, config);
    let (response, mut handler) = session::handle(&req, body)?;

    spawn(async move {
        let mut interval = time::interval(Duration::from_millis(STATE_CHECK_INTERVAL_MS));

        loop {
            tokio::select! {
                msg = handler.next() => {
                    match msg {
                        WebSocketMessage::Text(txt) => {
                            if let Err(e) = socket.dependencies(&txt).await {
                                let session = handler.session();
                                let _ = session.text("internal server error").await.inspect_err(|e| error!("{e}"));
                                error!("{e}");
                                handler.error();
                                break;
                            }
                        }
                        WebSocketMessage::Continue => {}
                        _ => break,
                    }
                }

                _ = interval.tick() => {
                    let session = handler.session();
                    socket.scan(session).await;
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
