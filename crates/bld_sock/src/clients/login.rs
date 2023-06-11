use std::{
    fmt::Write,
    process::{ExitStatus, Stdio},
};

use actix::{
    fut::{ready, WrapFuture},
    io::{SinkWrite, WriteHandler},
    Actor, ActorContext, ActorFutureExt, AsyncContext, Context, Handler, StreamHandler, System,
};
use actix_codec::Framed;
use anyhow::Result;
use awc::{
    error::WsProtocolError,
    ws::{Codec, Frame, Message},
    BoxedSocket,
};
use bld_config::{os_name, OSname};
use bld_core::{
    auth::write_tokens,
    messages::{LoginClientMessage, LoginServerMessage},
};
use futures::stream::SplitSink;
use tokio::process::Command;
use tracing::error;

pub struct LoginClient {
    server: String,
    writer: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>,
}

impl LoginClient {
    pub fn new(
        server: String,
        writer: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>,
    ) -> Self {
        Self { server, writer }
    }

    fn handle_server_message(
        &mut self,
        message: &str,
        ctx: &mut <Self as Actor>::Context,
    ) -> Result<()> {
        let message: LoginServerMessage = serde_json::from_str(message)?;
        match message {
            LoginServerMessage::AuthorizationUrl(url) => {
                println!("Opening a new browser tab to start the login process.");

                let mut command = match os_name() {
                    OSname::Linux => Command::new("xdg-open"),
                    _ => unimplemented!(),
                };
                command.arg(&url);
                command.stdout(Stdio::null());
                command.stderr(Stdio::null());

                let status_fut = command
                    .status()
                    .into_actor(self)
                    .then(move |res, _, _| {
                        let success = res
                            .map(|x| ExitStatus::success(&x))
                            .unwrap_or_default();
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
                if let Err(e) = write_tokens(&self.server, tokens) {
                    println!("Login failed, {e}");
                } else {
                    println!("Login completed successfully!");
                }
                ctx.stop();
            }

            LoginServerMessage::Failed { reason } => {
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
                let message = String::from_utf8_lossy(&bt[..]);
                let _ = self
                    .handle_server_message(&message, ctx)
                    .map_err(|e| error!("{e}"));
            }
            Ok(Frame::Close(_)) => {
                ctx.stop();
            }
            _ => {}
        }
    }
}

impl WriteHandler<WsProtocolError> for LoginClient {}
