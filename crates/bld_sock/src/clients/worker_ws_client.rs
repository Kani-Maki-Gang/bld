use anyhow::Result;
use awc::ws::Frame;
use bld_config::BldConfig;
use bld_core::logger::Logger;
use bld_http::WebSock;
use bld_models::dtos::WorkerMessages;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tracing::debug;

pub struct WorkerClient {
    logger: Logger,
    sock: WebSock,
}

impl WorkerClient {
    pub async fn connect(config: Arc<BldConfig>, logger: Logger) -> Result<Self> {
        let url = format!("{}/v1/ws-worker/", config.local.supervisor.base_url_ws());
        debug!("establishing web socket connection on {}", url);
        let sock = WebSock::connect(&url, None).await?;
        Ok(Self { logger, sock })
    }

    pub async fn run(mut self, mut worker_rx: Receiver<WorkerMessages>) -> Result<()> {
        self.sock.binary(&WorkerMessages::Ack).await?;
        self.sock
            .binary(&WorkerMessages::WhoAmI {
                pid: std::process::id(),
            })
            .await?;

        loop {
            tokio::select! {
                msg = worker_rx.recv() => {
                    // a closed channel means the runner is done, exit instead of
                    // keeping the process alive until the supervisor closes the socket
                    let Some(msg) = msg else {
                        break;
                    };
                    debug!("sending message to supervisor {:?}", msg);
                    if self.sock.binary(&msg).await.is_err() {
                        break;
                    }
                }
                res = self.sock.next() => {
                    let Ok(frame) = res else {
                        break;
                    };
                    match frame {
                        Frame::Text(bt) => {
                            let message = format!("{}", String::from_utf8_lossy(&bt));
                            self.logger.write_line(message).await?;
                        }
                        Frame::Close(_) => break,
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }
}
