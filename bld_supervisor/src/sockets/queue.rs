use crate::{
    base::{Queue, ServerMessages},
    queues::WorkerQueue,
};
use actix::prelude::*;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use bld_core::workers::PipelineWorker;
use std::env::current_exe;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::{debug, error, info};

pub struct QueueWorkerSocket {
    hb: Instant,
    worker_queue: web::Data<Mutex<WorkerQueue>>,
}

impl QueueWorkerSocket {
    pub fn new(worker_queue: web::Data<Mutex<WorkerQueue>>) -> Self {
        Self {
            hb: Instant::now(),
            worker_queue,
        }
    }

    fn heartbeat(_act: &Self, ctx: &mut <Self as Actor>::Context) {
        // if Instant::now().duration_since(act.hb) > Duration::from_secs(10) {
        //     info!("queue websocket heartbeat failed, disconnecting");
        //     ctx.stop();
        //     return;
        // }
        ctx.ping(b"");
    }

    fn handle_message(&self, bytes: &web::Bytes) -> anyhow::Result<()> {
        let msg: ServerMessages = serde_json::from_slice(&bytes[..])?;
        match msg {
            ServerMessages::Ack => info!("a new server connection was acknowledged"),
            ServerMessages::Enqueue {
                pipeline,
                run_id,
                variables,
                environment,
            } =>
            {
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
                    command.arg("--variables");
                    command.arg(&variables);
                }
                if let Some(environment) = environment {
                    command.arg("--environment");
                    command.arg(&environment);
                }
                info!("command for pipeline: {pipeline} was built successfully");
                let mut queue = self.worker_queue.lock().unwrap();
                queue.enqueue(PipelineWorker::new(command));
                info!("worker for pipeline: {pipeline} has been queued");
            }
        }
        Ok(())
    }
}

impl Actor for QueueWorkerSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(Duration::from_millis(500), |act, ctx| {
            QueueWorkerSocket::heartbeat(act, ctx);
        });
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for QueueWorkerSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Binary(bytes)) => {
                debug!("received binary message");
                let _ = self.handle_message(&bytes);
            }
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

pub async fn ws_queue_worker(
    req: HttpRequest,
    stream: web::Payload,
    worker_queue: web::Data<Mutex<WorkerQueue>>,
) -> Result<HttpResponse, Error> {
    let socket = QueueWorkerSocket::new(worker_queue);
    ws::start(socket, &req, stream)
}
