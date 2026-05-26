use crate::queues::WorkerQueueSender;
use actix_web::{
    HttpRequest, Responder,
    web::{self, Bytes, Data},
};
use anyhow::Result;
use bld_core::workers::Worker;
use bld_models::dtos::ServerMessages;
use bld_sock::session::{self, WebSocketMessage};
use std::env::current_exe;
use tokio::process::Command;
use tracing::{debug, error, info};

async fn handle_message(worker_queue_tx: &Data<WorkerQueueSender>, bytes: &Bytes) -> Result<()> {
    let msg: ServerMessages = serde_json::from_slice(&bytes[..])?;
    match msg {
        ServerMessages::Ack => info!("a new server connection was acknowledged"),

        ServerMessages::Enqueue {
            pipeline,
            run_id,
            inputs,
            env,
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
            if let Some(inputs) = inputs {
                for entry in inputs {
                    command.arg("--input");
                    command.arg(entry);
                }
            }
            if let Some(env) = env {
                for entry in env {
                    command.arg("--environment");
                    command.arg(entry);
                }
            }

            let worker = Box::new(Worker::new(run_id, command));
            worker_queue_tx
                .enqueue(worker)
                .await
                .inspect(|_| info!("worker for pipeline: {pipeline} has been queued"))?;
        }

        ServerMessages::Stop { run_id } => {
            info!("server sent a stop message for run_id: {run_id}");
            worker_queue_tx
                .stop(&run_id)
                .await
                .inspect(|_| info!("stop signal sent to worker for run_id: {run_id}"))?;
        }
    }
    Ok(())
}

pub async fn ws(
    req: HttpRequest,
    body: web::Payload,
    worker_queue_tx: Data<WorkerQueueSender>,
) -> actix_web::Result<impl Responder> {
    let (response, mut handler) = session::handle(&req, body)?;

    actix_web::rt::spawn(async move {
        loop {
            match handler.next().await {
                WebSocketMessage::Binary(bytes) => {
                    debug!("received binary message from server");
                    if let Err(e) = handle_message(&worker_queue_tx, &bytes).await {
                        let session = handler.session();
                        let _ = session
                            .text("internal server error")
                            .await
                            .inspect_err(|e| error!("{e}"));
                        error!("handling message error. {e}");
                        handler.error();
                    }
                }
                WebSocketMessage::Continue => {}
                _ => break,
            }
        }

        handler.cleanup().await;
    });

    Ok(response)
}
