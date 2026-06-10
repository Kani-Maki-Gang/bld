use std::sync::Arc;

use anyhow::Result;
use awc::ws::Frame;
use bld_config::BldConfig;
use bld_core::context::Context;
use bld_core::logger::Logger;
use bld_http::WebSock;
use bld_models::dtos::{ExecClientMessage, ExecServerMessage};
use tracing::{debug, error};

pub struct ExecClient {
    run_id: Option<String>,
    server: String,
    logger: Arc<Logger>,
    context: Arc<Context>,
    sock: WebSock,
}

impl ExecClient {
    pub async fn connect(
        config: Arc<BldConfig>,
        server: String,
        logger: Arc<Logger>,
        context: Arc<Context>,
    ) -> Result<Self> {
        let server_config = config.server(&server)?;
        let auth_path = config.auth_full_path(&server_config.name);
        let url = format!("{}/v1/ws-exec/", server_config.base_url_ws());

        debug!("establishing web socket connection on {}", url);

        let sock = WebSock::connect(&url, Some(&auth_path)).await?;

        Ok(Self {
            run_id: None,
            server,
            logger,
            context,
            sock,
        })
    }

    pub async fn run(mut self, message: ExecClientMessage) -> Result<()> {
        debug!("sending message to socket: {:?}", message);
        self.sock.text(&message).await?;

        loop {
            let Ok(frame) = self.sock.next().await else {
                break;
            };
            match frame {
                Frame::Text(bt) => {
                    let message = String::from_utf8_lossy(&bt[..]);
                    let _ = self
                        .handle_server_message(&message)
                        .await
                        .map_err(|e| error!("{e}"));
                }
                Frame::Close(_) => break,
                _ => {}
            }
        }

        if let Some(run_id) = &self.run_id {
            let _ = self
                .context
                .remove_remote_run(run_id)
                .await
                .map_err(|e| error!("{e}"));
        }

        Ok(())
    }

    async fn handle_server_message(&mut self, message: &str) -> Result<()> {
        let message: ExecServerMessage = serde_json::from_str(message)?;

        match message {
            ExecServerMessage::QueuedRun { run_id } => {
                self.run_id = Some(run_id.to_owned());
                self.context
                    .add_remote_run(self.server.to_owned(), run_id)
                    .await?;
            }

            ExecServerMessage::Log { content } => {
                self.logger.write_line(content).await?;
            }
        }

        Ok(())
    }
}
