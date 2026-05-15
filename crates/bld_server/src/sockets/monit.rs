use crate::extractors::User;
use actix_web::{
    HttpRequest, Responder, error::ErrorUnauthorized, rt::{spawn, time}, web::{Data, Payload}
};
use actix_ws::{CloseReason, Message, Session, handle};
use anyhow::{Result, bail};
use bld_config::BldConfig;
use bld_core::scanner::FileScanner;
use bld_models::{
    dtos::MonitInfo,
    pipeline_runs::{self, PR_STATE_FAULTED, PR_STATE_FINISHED},
};
use futures::StreamExt;
use sea_orm::DatabaseConnection;
use std::time::Duration;
use tracing::{debug, error};

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

    async fn exec(&self, session: &mut Session) -> bool {
        match pipeline_runs::select_by_id(self.conn.as_ref(), &self.id).await {
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

    async fn dependencies(&mut self, session: &mut Session, data: &str) -> Result<()> {
        let conn = self.conn.clone();
        let data = serde_json::from_str::<MonitInfo>(data)?;
        let run = if data.last {
            pipeline_runs::select_last(conn.as_ref()).await
        } else if let Some(id) = data.id {
            pipeline_runs::select_by_id(conn.as_ref(), &id).await
        } else if let Some(name) = data.name {
            pipeline_runs::select_by_name(conn.as_ref(), &name).await
        } else {
            bail!("file not found");
        };
        match run {
            Ok(run) => {
                debug!("starting scan for run");
                self.id.clone_from(&run.id);
                self.scanner = Some(FileScanner::new(self.config.as_ref(), &run.id));
                Ok(())
            }
            Err(e) => {
                session.text("internal server error").await?;
                Err(e)
            }
        }
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
    let (response, mut session, mut msg_stream) = handle(&req, body)?;

    spawn(async move {
        let mut reason: Option<CloseReason> = None;
        let mut scan_interval = time::interval(Duration::from_millis(500));
        let mut exec_interval = time::interval(Duration::from_millis(500));

        loop {
            tokio::select! {
                Some(msg) = msg_stream.next() => {
                    match msg {
                        Ok(Message::Text(txt)) => {
                            if let Err(e) = socket.dependencies(&mut session, &txt).await {
                                error!("{e}");
                                break;
                            }
                        }
                        Ok(Message::Ping(msg)) => {
                            if let Err(e) = session.pong(&msg).await {
                                error!("{e}");
                                break;
                            }
                        }
                        Ok(Message::Pong(msg)) => {
                            if let Err(e) = session.ping(&msg).await {
                                error!("{e}");
                                break;
                            }
                        }
                        Ok(Message::Close(r)) => {
                            reason = r;
                            break;
                        }
                        Err(e) => {
                            error!("{e}");
                            break;
                        }
                        _ => break,
                    }
                }

                _ = scan_interval.tick() => {
                    socket.scan(&mut session).await;
                }

                _ = exec_interval.tick() => {
                    if !socket.exec(&mut session).await {
                        break;
                    }
                }
            }
        }

        if let Err(e) = session.close(reason).await {
            error!("encountered error while closing websocket session due to {e}");
        }
    });

    Ok(response)
}
