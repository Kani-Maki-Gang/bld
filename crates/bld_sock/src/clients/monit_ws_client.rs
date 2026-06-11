use anyhow::Result;
use awc::ws::Frame;
use bld_config::BldConfig;
use bld_core::logger::Logger;
use bld_http::WebSock;
use bld_models::dtos::MonitInfo;
use tracing::debug;

pub struct MonitClient {
    logger: Logger,
    sock: WebSock,
}

impl MonitClient {
    pub async fn connect(config: BldConfig, logger: Logger, server: String) -> Result<Self> {
        let server_config = config.server(&server)?;
        let auth_path = config.auth_full_path(&server_config.name);
        let url = format!("{}/v1/ws-monit/", server_config.base_url_ws());
        debug!("establishing web socket connection on {}", url);
        let sock = WebSock::connect(&url, Some(&auth_path)).await?;
        Ok(Self { logger, sock })
    }

    pub async fn run(mut self, info: MonitInfo) -> Result<()> {
        debug!("sending monit info to socket");
        self.sock.text(&info).await?;

        while let Ok(frame) = self.sock.next().await {
            match frame {
                Frame::Text(bt) => {
                    self.logger
                        .write_line(String::from_utf8_lossy(&bt).into())
                        .await?;
                }
                Frame::Close(_) => break,
                _ => {}
            }
        }

        Ok(())
    }
}
