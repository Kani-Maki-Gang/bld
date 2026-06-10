use anyhow::Result;
use awc::ws::Frame;
use bld_core::logger::Logger;
use bld_http::WebSock;
use bld_models::dtos::ServerMessages;
use tracing::{debug, error, info};

pub enum EnqueueClientState {
    Continue,
    Completed,
}

pub struct EnqueueClient {
    logger: Logger,
    sock: WebSock,
}

impl EnqueueClient {
    pub async fn connect(url: &str, logger: Logger) -> Result<Self> {
        debug!("establishing web socket connection on {}", url);
        let mut sock = WebSock::connect(url, None).await?;
        sock.binary(&ServerMessages::Ack).await?;
        Ok(Self { logger, sock })
    }

    pub async fn send(&mut self, message: &ServerMessages) -> Result<()> {
        self.sock.binary(message).await
    }

    pub async fn next(&mut self) -> EnqueueClientState {
        match self.sock.next().await {
            Ok(Frame::Text(bt)) => {
                let _ = self
                    .logger
                    .write_line(String::from_utf8_lossy(&bt).into())
                    .await
                    .inspect_err(|e| error!("unable to log line due to {e}"));
                EnqueueClientState::Continue
            }
            Ok(Frame::Close(_)) => {
                info!("web socket connection stopped due to a sent closed frame");
                EnqueueClientState::Completed
            }
            Ok(_) => EnqueueClientState::Continue,
            Err(e) => {
                error!("{e}");
                EnqueueClientState::Completed
            }
        }
    }
}
