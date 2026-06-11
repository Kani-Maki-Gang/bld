use std::{
    fmt::Write,
    process::{ExitStatus, Stdio},
};

use anyhow::{Result, bail};
use awc::ws::Frame;
use bld_config::{BldConfig, OSname, os_name};
use bld_core::logger::Logger;
use bld_http::WebSock;
use bld_models::dtos::{LoginClientMessage, LoginServerMessage};
use bld_utils::fs::write_tokens;
use tokio::process::Command;
use tracing::{debug, error};

pub struct LoginClient {
    config: BldConfig,
    logger: Logger,
    server: String,
    sock: WebSock,
}

impl LoginClient {
    pub async fn connect(config: BldConfig, logger: Logger, server: String) -> Result<Self> {
        let server_config = config.server(&server)?;
        let auth_path = config.auth_full_path(&server_config.name);
        let url = format!("{}/v1/ws-login/", server_config.base_url_ws());

        debug!("establishing web socket connection on {}", url);

        let sock = WebSock::connect(&url, Some(&auth_path)).await?;

        Ok(Self {
            config,
            logger,
            server,
            sock,
        })
    }

    pub async fn run(mut self) -> Result<()> {
        self.sock.text(&LoginClientMessage::Init).await?;

        while let Ok(frame) = self.sock.next().await {
            match frame {
                Frame::Text(bt) => {
                    debug!("received text message from server");
                    let message = String::from_utf8_lossy(&bt[..]);
                    if self.handle_server_message(&message).await? {
                        break;
                    }
                }
                Frame::Close(_) => {
                    debug!("received close message from server");
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn handle_server_message(&self, message: &str) -> Result<bool> {
        let Ok(message) = serde_json::from_str::<LoginServerMessage>(message)
            .inspect_err(|e| error!("unable to parse server message, {e}"))
        else {
            return Ok(false);
        };

        match message {
            LoginServerMessage::AuthorizationUrl(url) => {
                debug!("Received message to open url for the login process to begin");
                debug!("Opening browser with url: {url}");
                self.logger
                    .write_line("Opening a new browser tab to start the login process.".to_string())
                    .await?;

                let (command, args) = match os_name() {
                    OSname::Linux => ("xdg-open", vec![url.as_str()]),
                    OSname::Windows => ("powershell", vec!["-c", "Start-Process", url.as_str()]),
                    _ => unimplemented!(),
                };
                let mut command = Command::new(command);
                command.args(args);
                command.stdout(Stdio::null());
                command.stderr(Stdio::null());

                let success = command
                    .status()
                    .await
                    .as_ref()
                    .map(ExitStatus::success)
                    .unwrap_or_default();
                if !success {
                    let mut message = String::new();
                    let _ = writeln!(
                        message,
                        "Couldn't open the browser, please use the below url in order to login:"
                    );
                    let _ = write!(message, "{url}");
                    self.logger.write_line(message).await?;
                }
                Ok(false)
            }

            LoginServerMessage::Completed(tokens) => {
                debug!("login process completed writing tokens to disk");
                let auth_path = self.config.auth_full_path(&self.server);
                if let Err(e) = write_tokens(&auth_path, tokens).await {
                    error!("unable to write tokens to disk due to: {e}");
                    println!("Login failed, {e}");
                    bail!("login failed, {e}");
                }
                debug!("wrote tokens to disk successfully");
                self.logger
                    .write_line("Login completed successfully!".to_string())
                    .await?;
                Ok(true)
            }

            LoginServerMessage::Failed(reason) => {
                let message = format!("Login failed, {reason}");
                self.logger.write_line(message.clone()).await?;
                bail!(message)
            }
        }
    }
}
