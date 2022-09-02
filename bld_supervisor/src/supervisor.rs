use crate::queues::WorkerQueue;
use crate::sockets::{ws_active_worker, ws_queue_worker};
use actix_web::{web, App, HttpServer};
use anyhow::anyhow;
use bld_config::BldConfig;
// use bld_config::{path, BldConfig};
// use std::env::temp_dir;
// use std::path::PathBuf;
use std::sync::Mutex;

pub async fn start(_config: BldConfig) -> anyhow::Result<()> {
    // let socket = path![temp_dir(), config.local.unix_sock];
    let url = format!("127.0.0.1:7000");
    let worker_queue = web::Data::new(Mutex::new(WorkerQueue::new(10)));
    HttpServer::new(move || {
        App::new()
            .app_data(worker_queue.clone())
            .service(web::resource("/ws-queue/").route(web::get().to(ws_queue_worker)))
            .service(web::resource("/ws-worker/").route(web::get().to(ws_active_worker)))
    })
    // .bind_uds(socket)?
    .bind(url)?
    .run()
    .await
    .map_err(|e| anyhow!(e))
}
