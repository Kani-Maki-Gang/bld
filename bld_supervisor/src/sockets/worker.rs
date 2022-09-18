use crate::{
    base::{Queue, WorkerMessages},
    queues::WorkerQueue,
};
use actix::prelude::*;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use std::sync::Mutex;
use tracing::{debug, error, info};

pub struct ActiveWorkerSocket {
    worker_pid: Option<u32>,
    worker_queue: web::Data<Mutex<WorkerQueue>>,
}

impl ActiveWorkerSocket {
    pub fn new(worker_queue: web::Data<Mutex<WorkerQueue>>) -> Self {
        Self {
            worker_pid: None,
            worker_queue,
        }
    }

    fn handle_message(
        &mut self,
        bytes: &web::Bytes,
        ctx: &mut <Self as Actor>::Context,
    ) -> anyhow::Result<()> {
        let msg: WorkerMessages = serde_json::from_slice(&bytes[..])?;
        match msg {
            WorkerMessages::Ack => info!("a new worker connection was acknowledged"),
            WorkerMessages::WhoAmI { pid } => {
                info!("worker with pid: {pid} sent a whoami message");
                self.worker_pid = Some(pid);
            }
            WorkerMessages::Completed => {
                info!("worker just completed, starting cleanup");
                self.cleanup(ctx);
            }
        }
        Ok(())
    }

    fn cleanup(&self, ctx: &mut <Self as Actor>::Context) {
        if let Some(pid) = self.worker_pid {
            let mut queue = self.worker_queue.lock().unwrap();
            if let Err(e) = queue.dequeue(pid) {
                error!("{e}");
            }
        }
        ctx.stop();
    }
}

impl Actor for ActiveWorkerSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("active worker socket started");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        debug!("active worker socket stopped");
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ActiveWorkerSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Binary(bytes)) => {
                let _ = self.handle_message(&bytes, ctx);
            }
            Ok(ws::Message::Ping(msg)) => {
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {}
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                self.cleanup(ctx);
            }
            _ => self.cleanup(ctx),
        }
    }
}

pub async fn ws_active_worker(
    req: HttpRequest,
    stream: web::Payload,
    worker_queue: web::Data<Mutex<WorkerQueue>>,
) -> Result<HttpResponse, Error> {
    let socket = ActiveWorkerSocket::new(worker_queue);
    ws::start(socket, &req, stream)
}
