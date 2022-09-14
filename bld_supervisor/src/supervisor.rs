use crate::queues::WorkerQueue;
use crate::sockets::{ws_active_worker, ws_queue_worker};
use actix_web::{web, App, HttpServer};
use anyhow::anyhow;
use bld_config::BldConfig;
use std::sync::Mutex;

pub async fn start(config: BldConfig) -> anyhow::Result<()> {
    let url = format!("{}:{}", config.local.supervisor.host, config.local.supervisor.port);
    let worker_queue = web::Data::new(Mutex::new(WorkerQueue::new(config.local.supervisor.workers.try_into()?)));
    HttpServer::new(move || {
        App::new()
            .app_data(worker_queue.clone())
            .service(web::resource("/ws-queue/").route(web::get().to(ws_queue_worker)))
            .service(web::resource("/ws-worker/").route(web::get().to(ws_active_worker)))
    })
    .bind(url)?
    .run()
    .await
    .map_err(|e| anyhow!(e))
}
