use std::{
    fmt::Write,
    process::{ExitStatus, Stdio},
    sync::Arc,
};

use actix::{
    Actor, ActorContext, ActorFutureExt, AsyncContext, Context, Handler, StreamHandler, System,
    fut::WrapFuture,
    io::{SinkWrite, WriteHandler},
};
use actix_codec::Framed;
use anyhow::Result;
use awc::{
    BoxedSocket,
    error::WsProtocolError,
    ws::{Codec, Frame, Message},
};
use bld_config::{BldConfig, OSname, os_name};
use bld_models::dtos::{LoginClientMessage, LoginServerMessage};
use bld_utils::fs::write_tokens;
use futures::stream::SplitSink;
use futures_util::future::ready;
use tokio::process::Command;
use tracing::{debug, error};

pub struct LoginClient {
    config: Arc<BldConfig>,
    server: String,
    writer: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>,
}

impl LoginClient {
    pub fn new(
        config: Arc<BldConfig>,
        server: String,
        writer: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>,
    ) -> Self {
        Self {
            config,
            server,
            writer,
        }
    }

    fn handle_server_message(
        &mut self,
        message: &str,
        ctx: &mut <Self as Actor>::Context,
    ) -> Result<()> {
        let message: LoginServerMessage = serde_json::from_str(message)?;
        match message {
            LoginServerMessage::AuthorizationUrl(url) => {
                debug!("received message to open url for the login process to begin");
                debug!("Opening browser with url: {url}");
                println!("Opening a new browser tab to start the login process.");

                let (command, args) = match os_name() {
                    OSname::Linux => ("xdg-open", vec![url.as_str()]),
                    OSname::Windows => ("powershell", vec!["-c", "Start-Process", url.as_str()]),
                    _ => unimplemented!(),
                };
                let mut command = Command::new(command);
                command.args(args);
                command.stdout(Stdio::null());
                command.stderr(Stdio::null());

                let status_fut = command.status().into_actor(self).then(move |res, _, _| {
                    let success = res.as_ref().map(ExitStatus::success).unwrap_or_default();
                    debug!("browser process was closed with exit status: {res:?}");
                    if !success {
                        let mut message = String::new();
                        let _ = writeln!(
                            message,
                            "Couldn't open the browser, please use the below url in order to login:"
                        );
                        let _ = write!(message, "{url}");
                        println!("{message}");
                    }
                    ready(())
                });
                ctx.spawn(status_fut);
            }

            LoginServerMessage::Completed(tokens) => {
                debug!("login process completed writing tokens to disk");
                let auth_path = self.config.auth_full_path(&self.server);
                let write_fut = async move { write_tokens(&auth_path, tokens).await }
                    .into_actor(self)
                    .then(|res, _, ctx| {
                        if let Err(e) = res {
                            error!("unable to write tokens to disk due to: {e}");
                            println!("Login failed, {e}");
                        } else {
                            debug!("wrote tokens to disk successfully");
                            println!("Login completed successfully!");
                        }
                        ctx.stop();
                        ready(())
                    });

                ctx.spawn(write_fut);
            }

            LoginServerMessage::Failed(reason) => {
                println!("Login failed, {reason}");
                ctx.stop();
            }
        }
        Ok(())
    }
}

impl Actor for LoginClient {
    type Context = Context<Self>;

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        debug!("web socket connection stopped");
        if let Some(system) = System::try_current() {
            system.stop();
        }
    }
}

impl Handler<LoginClientMessage> for LoginClient {
    type Result = ();

    fn handle(&mut self, msg: LoginClientMessage, _ctx: &mut Self::Context) {
        if let Ok(msg) = serde_json::to_string(&msg) {
            let _ = self.writer.write(Message::Text(msg.into()));
        }
    }
}

impl StreamHandler<Result<Frame, WsProtocolError>> for LoginClient {
    fn handle(&mut self, item: Result<Frame, WsProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(Frame::Text(bt)) => {
                debug!("received test message from server");
                let message = String::from_utf8_lossy(&bt[..]);
                let _ = self
                    .handle_server_message(&message, ctx)
                    .map_err(|e| error!("{e}"));
            }
            Ok(Frame::Close(_)) => {
                debug!("received close message from server");
                ctx.stop();
            }
            _ => {}
        }
    }
}

impl WriteHandler<WsProtocolError> for LoginClient {}
