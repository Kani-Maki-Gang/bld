use crate::base::Queue;
use crate::queues::WorkerQueue;
use actix::prelude::*;
use actix_web::web::{Bytes, Data, Payload};
use actix_web::{Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use bld_core::workers::PipelineWorker;
use bld_sock::messages::ServerMessages;
use std::env::current_exe;
use std::process::Command;
use std::sync::Mutex;
use tracing::{debug, error, info};

pub struct ServerSocket {
    worker_queue: Data<Mutex<WorkerQueue>>,
}

impl ServerSocket {
    pub fn new(worker_queue: Data<Mutex<WorkerQueue>>) -> Self {
        Self { worker_queue }
    }

    fn handle_message(&self, bytes: &Bytes) -> anyhow::Result<()> {
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
                let mut queue = self.worker_queue.lock().unwrap();
                queue.enqueue(PipelineWorker::new(run_id, command))?;
                info!("worker for pipeline: {pipeline} has been queued");
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
    worker_queue: Data<Mutex<WorkerQueue>>,
) -> Result<HttpResponse, Error> {
    let socket = ServerSocket::new(worker_queue);
    ws::start(socket, &req, stream)
}
