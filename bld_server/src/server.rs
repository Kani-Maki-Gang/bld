use crate::endpoints::{
    auth_redirect, deps, ha_append_entries, ha_install_snapshot, ha_vote, hist, home, inspect,
    list, pull, push, remove, stop,
};
use crate::queue::EnqueueClient;
use crate::sockets::{ws_exec, ws_high_avail, ws_monit};
use actix::{Actor, io::SinkWrite, StreamHandler};
use actix_web::{middleware, web, App, HttpServer};
use anyhow::anyhow;
use awc::Client;
use bld_config::{path, BldConfig};
use bld_core::database::new_connection_pool;
use bld_core::high_avail::HighAvail;
use bld_core::proxies::ServerPipelineProxy;
use bld_supervisor::base::ServerMessages;
use futures::stream::StreamExt;
use std::env::{set_var, temp_dir};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info};

async fn start_web_server(
    config: BldConfig,
    host: &str,
    port: i64,
    enqueue_tx: Mutex<Sender<ServerMessages>>,
) -> anyhow::Result<()> {
    info!("starting bld server at {}:{}", host, port);
    let pool = new_connection_pool(&config.local.db)?;
    let enqueue_tx = web::Data::new(enqueue_tx);
    let ha = web::Data::new(HighAvail::new(&config, pool.clone()).await?);
    let pool = web::Data::new(pool);
    let cfg = web::Data::new(config);
    let prx = web::Data::new(ServerPipelineProxy::new(
        Arc::clone(&cfg),
        Arc::clone(&pool),
    ));
    set_var("RUST_LOG", "actix_server=info,actix_web=debug");
    HttpServer::new(move || {
        App::new()
            .app_data(cfg.clone())
            .app_data(enqueue_tx.clone())
            .app_data(ha.clone())
            .app_data(pool.clone())
            .app_data(prx.clone())
            .wrap(middleware::Logger::default())
            .service(ha_append_entries)
            .service(ha_install_snapshot)
            .service(ha_vote)
            .service(home)
            .service(auth_redirect)
            .service(hist)
            .service(list)
            .service(remove)
            .service(push)
            .service(deps)
            .service(pull)
            .service(stop)
            .service(inspect)
            .service(web::resource("/ws-exec/").route(web::get().to(ws_exec)))
            .service(web::resource("/ws-monit/").route(web::get().to(ws_monit)))
            .service(web::resource("/ws-ha/").route(web::get().to(ws_high_avail)))
    })
    .bind(format!("{host}:{port}"))?
    .run()
    .await?;
    Ok(())
}

async fn start_supervisor_socket(
    _socket_path: String,
    enqueue_rx: Receiver<ServerMessages>,
) -> anyhow::Result<()>
{
    // let url = format!("unix:/{socket_path}/ws-queue/");
    let url = format!("http://127.0.0.1:7000/ws-queue/");
    debug!("establishing web socket connection on {}", url);
    let (_, framed) = Client::new().ws(url).connect().await.map_err(|e| {
        error!("{e}");
        anyhow!(e.to_string())
    })?;
    let (sink, stream) = framed.split();
    let addr = EnqueueClient::create(|ctx| {
        EnqueueClient::add_stream(stream, ctx);
        EnqueueClient::new(SinkWrite::new(sink, ctx))
    });
    addr.send(ServerMessages::Ack).await?;
    while let Ok(msg) = enqueue_rx.recv(){
        addr.send(msg).await?;
    }
    Ok(())
}

pub async fn start(
    config: BldConfig,
    host: &str,
    port: i64,
) -> anyhow::Result<()> {
    let socket_path = path![temp_dir(), &config.local.unix_sock].display().to_string();
    let (enqueue_tx, enqueue_rx) = channel::<ServerMessages>();
    let enqueue_tx = Mutex::new(enqueue_tx);
    let _ = tokio::join!(
        start_web_server(config, host, port, enqueue_tx),
        start_supervisor_socket(socket_path, enqueue_rx),
    );
    Ok(())
}
