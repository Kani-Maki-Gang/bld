use std::{
    fmt::Write as FmtWrite,
    fs::{create_dir_all, remove_file, File},
    io::Write,
    path::PathBuf,
    process::{Command, ExitStatus, Stdio},
};

use actix::{
    io::{SinkWrite, WriteHandler},
    Actor, ActorContext, Context, Handler, StreamHandler, System,
};
use actix_codec::Framed;
use anyhow::Result;
use awc::{
    error::WsProtocolError,
    ws::{Codec, Frame, Message},
    BoxedSocket,
};
use bld_config::{definitions::REMOTE_SERVER_AUTH, os_name, path, OSname};
use futures::stream::SplitSink;
use tracing::error;

use crate::messages::{LoginClientMessage, LoginServerMessage};

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

                let status = command.status()?;
                if !ExitStatus::success(&status) {
                    let mut message = String::new();
                    writeln!(
                        message,
                        "Couldn't open the browser, please use the below url in order to login:"
                    )?;
                    write!(message, "{url}")?;
                    println!("{message}");
                }
            }

            LoginServerMessage::Completed { access_token } => {
                let mut path = path![REMOTE_SERVER_AUTH];

                create_dir_all(&path)?;

                path.push(&self.server);
                if path.is_file() {
                    remove_file(&path)?;
                }

                File::create(path)?.write_all(access_token.as_bytes())?;
                println!("Login completed successfully");
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
