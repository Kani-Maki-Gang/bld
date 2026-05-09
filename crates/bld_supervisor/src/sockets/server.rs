use crate::queues::WorkerQueueSender;
use actix_http::ws::Message;
use actix_web::{
    HttpRequest, Responder,
    web::{self, Bytes, Data},
};
use actix_ws::CloseReason;
use anyhow::Result;
use bld_core::workers::Worker;
use bld_models::dtos::ServerMessages;
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
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;

    actix_web::rt::spawn(async move {
        let mut reason: Option<CloseReason> = None;
        while let Some(Ok(msg)) = msg_stream.recv().await {
            match msg {
                Message::Binary(bytes) => {
                    debug!("received binary message from server");
                    if let Err(e) = handle_message(&worker_queue_tx, &bytes).await {
                        error!("handling message error. {e}");
                    }
                }
                Message::Ping(msg) => {
                    if let Err(e) = session.pong(&msg).await {
                        error!("{e}");
                        break;
                    }
                }
                Message::Pong(_) => {}
                Message::Close(r) => {
                    reason = r;
                    break;
                }
                _ => {
                    break;
                }
            }
        }

        if let Err(e) = session.close(reason).await {
            error!("encountered error while closing websocket session due to {e}");
        }
    });

    Ok(response)
}
