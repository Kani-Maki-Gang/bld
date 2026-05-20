use crate::extractors::User;
use actix_web::{
    HttpRequest, Responder,
    error::ErrorUnauthorized,
    rt::{spawn, time},
    web::{Data, Payload},
};
use actix_ws::{CloseCode, CloseReason, Message, Session, handle};
use anyhow::{Result, bail};
use bld_config::BldConfig;
use bld_core::scanner::FileScanner;
use bld_models::{
    dtos::MonitInfo,
    pipeline_runs::{self, PR_STATE_FAULTED, PR_STATE_FINISHED},
};
use futures::StreamExt;
use sea_orm::DatabaseConnection;
use std::time::{Duration, Instant};
use tracing::{debug, error, warn};

const STATE_CHECK_INTERVAL_MS: u64 = 500;
const HEARTBEAT_INTERVAL_MS: u64 = 5_000;
const CLIENT_TIMEOUT_MS: u64 = 15_000;

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

    async fn check_state(&self, session: &mut Session) -> bool {
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

    async fn dependencies(&mut self, data: &str) -> Result<()> {
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
        }?;
        debug!("starting scan for run");
        self.id.clone_from(&run.id);
        self.scanner = Some(FileScanner::new(self.config.as_ref(), &run.id));
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
    let (response, mut session, mut msg_stream) = handle(&req, body)?;

    spawn(async move {
        let mut reason: Option<CloseReason> = None;
        let mut interval = time::interval(Duration::from_millis(STATE_CHECK_INTERVAL_MS));
        let mut hb_interval = time::interval(Duration::from_millis(HEARTBEAT_INTERVAL_MS));
        let mut last_pong = Instant::now();

        loop {
            tokio::select! {
                Some(msg) = msg_stream.next() => {
                    let Ok(msg) = msg.inspect_err(|e| error!("{e}")) else {
                        break;
                    };
                    match msg {
                        Message::Text(txt) => {
                            if let Err(e) = socket.dependencies(&txt).await {
                                reason = Some(CloseCode::Error.into());
                                let _ = session.text("internal server error").await.inspect_err(|e| error!("{e}"));
                                error!("{e}");
                                break;
                            }
                        }

                        Message::Ping(msg) => {
                            if let Err(e) = session.pong(&msg).await {
                                reason = Some(CloseCode::Error.into());
                                error!("{e}");
                                break;
                            }
                        }

                        Message::Pong(_) => {
                            last_pong = Instant::now();
                        }

                        Message::Continuation(_) | Message::Nop => {}

                        Message::Close(r) => {
                            reason = r;
                            break;
                        }

                        _ => break,
                    }
                }

                _ = interval.tick() => {
                    socket.scan(&mut session).await;
                    if !socket.check_state(&mut session).await {
                        break;
                    }
                }

                _ = hb_interval.tick() => {
                    if Instant::now().duration_since(last_pong)
                        > Duration::from_millis(CLIENT_TIMEOUT_MS)
                    {
                        warn!("client heartbeat timed out, closing session");
                        reason = Some(CloseCode::Away.into());
                        break;
                    }
                    if let Err(e) = session.ping(b"").await {
                        error!("ping failed: {e}");
                        reason = Some(CloseCode::Error.into());
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
