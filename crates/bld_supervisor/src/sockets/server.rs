use crate::queues::WorkerQueueSender;
use actix::prelude::*;
use actix_web::rt::spawn;
use actix_web::web::{Bytes, Data, Payload};
use actix_web::{Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use anyhow::Result;
use bld_core::workers::PipelineWorker;
use bld_sock::messages::ServerMessages;
use std::env::current_exe;
use std::process::Command;
use tracing::{debug, error, info};

pub struct ServerSocket {
    worker_queue_tx: Data<WorkerQueueSender>,
}

impl ServerSocket {
    pub fn new(worker_queue_tx: Data<WorkerQueueSender>) -> Self {
        Self { worker_queue_tx }
    }

    fn handle_message(&self, bytes: &Bytes) -> Result<()> {
        let msg: ServerMessages = serde_json::from_slice(&bytes[..])?;
        match msg {
            ServerMessages::Ack => info!("a new server connection was acknowledged"),

            ServerMessages::Enqueue {
                pipeline,
                run_id,
                variables,
                environment,
            } => {
                info!("server sent an enqueue message for pipeline: {pipeline}");
                let exe = current_exe().map_err(|e| {
                    error!("could not get the current executable. {e}");
                    e
                })?;
                let mut command = Command::new(exe);
                command.arg("worker");
                command.arg("--pipeline");
                command.arg(&pipeline);
                command.arg("--run-id");
                command.arg(&run_id);
                if let Some(variables) = variables {
                    for entry in variables {
                        command.arg("--variable");
                        command.arg(entry);
                    }
                }
                if let Some(environment) = environment {
                    for entry in environment {
                        command.arg("--environment");
                        command.arg(entry);
                    }
                }

                let tx = self.worker_queue_tx.clone();
                spawn(async move {
                    let _ = tx
                        .enqueue(PipelineWorker::new(run_id, command))
                        .await
                        .map(|_| info!("worker for pipeline: {pipeline} has been queued"))
                        .map_err(|e| error!("{e}"));
                });
            }

            ServerMessages::Stop { run_id } => {
                info!("server sent a stop message for run_id: {run_id}");

                let tx = self.worker_queue_tx.clone();
                spawn(async move {
                    let _ = tx
                        .stop(&run_id)
                        .await
                        .map(|_| info!("stop signal sent to worker for run_id: {run_id}"))
                        .map_err(|e| error!("{e}"));
                });
            }
        }
        Ok(())
    }
}

impl Actor for ServerSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("queue socket started");
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        info!("server connection stopped");
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ServerSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Binary(bytes)) => {
                debug!("received binary message from server");
                if let Err(e) = self.handle_message(&bytes) {
                    error!("handling message error. {e}");
                }
            }
            Ok(ws::Message::Ping(msg)) => {
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {}
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

pub async fn ws_server_socket(
    req: HttpRequest,
    stream: Payload,
    worker_queue_tx: Data<WorkerQueueSender>,
) -> Result<HttpResponse, Error> {
    let socket = ServerSocket::new(worker_queue_tx);
    ws::start(socket, &req, stream)
}
