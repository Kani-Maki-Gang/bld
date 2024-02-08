use actix::io::{SinkWrite, WriteHandler};
use actix::{Actor, ActorContext, Context as ActixContext, Handler, StreamHandler};
use actix_codec::Framed;
use actix_web::rt::{spawn, System};
use anyhow::Result;
use awc::error::WsProtocolError;
use awc::ws::{Codec, Frame, Message};
use awc::BoxedSocket;
use bld_core::context::Context;
use bld_core::logger::Logger;
use bld_models::dtos::{ExecClientMessage, ExecServerMessage};
use futures::stream::SplitSink;
use std::sync::Arc;
use tracing::{debug, error};

pub struct ExecClient {
    run_id: Option<String>,
    server: String,
    logger: Arc<Logger>,
    context: Arc<Context>,
    writer: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>,
}

impl ExecClient {
    pub fn new(
        server: String,
        logger: Arc<Logger>,
        context: Arc<Context>,
        writer: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>,
    ) -> Self {
        Self {
            run_id: None,
            server,
            logger,
            context,
            writer,
        }
    }

    fn handle_server_message(&mut self, message: &str) -> Result<()> {
        let message: ExecServerMessage = serde_json::from_str(message)?;

        match message {
            ExecServerMessage::QueuedRun { run_id } => {
                self.run_id = Some(run_id.to_owned());
                let server = self.server.to_owned();
                let context = self.context.clone();

                spawn(async move {
                    if let Err(e) = context.add_remote_run(server, run_id).await {
                        error!("{e}");
                    }
                });
            }

            ExecServerMessage::Log { content } => {
                let logger = self.logger.clone();
                spawn(async move {
                    if let Err(e) = logger.write_line(content).await {
                        error!("{e}");
                    }
                });
            }
        }

        Ok(())
    }
}

impl Actor for ExecClient {
    type Context = ActixContext<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("exec socket started");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        debug!("exec socket stopped");
        if let Some(current) = System::try_current() {
            current.stop();
        }
    }
}

impl Handler<ExecClientMessage> for ExecClient {
    type Result = ();

    fn handle(&mut self, msg: ExecClientMessage, _ctx: &mut Self::Context) {
        if let Ok(msg) = serde_json::to_string(&msg) {
            let _ = self.writer.write(Message::Text(msg.into()));
        }
    }
}

impl StreamHandler<Result<Frame, WsProtocolError>> for ExecClient {
    fn handle(&mut self, msg: Result<Frame, WsProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(Frame::Text(bt)) => {
                let message = format!("{}", String::from_utf8_lossy(&bt[..]));
                let _ = self
                    .handle_server_message(&message)
                    .map_err(|e| error!("{e}"));
            }
            Ok(Frame::Close(_)) => ctx.stop(),
            _ => {}
        }
    }

    fn finished(&mut self, ctx: &mut Self::Context) {
        if let Some(run_id) = &self.run_id {
            let context = self.context.clone();
            let run_id = run_id.clone();
            spawn(async move {
                let _ = context
                    .remove_remote_run(&run_id)
                    .await
                    .map_err(|e| error!("{e}"));
            });
        }
        ctx.stop();
    }
}

impl WriteHandler<WsProtocolError> for ExecClient {}
