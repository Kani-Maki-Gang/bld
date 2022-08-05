use crate::endpoints::{
    auth_redirect, deps, ha_append_entries, ha_install_snapshot, ha_vote, hist, home, inspect,
    list, pull, push, remove, stop,
};
use crate::sockets::{ws_exec, ws_high_avail, ws_monit};
use actix_web::{middleware, web, App, HttpServer};
use bld_config::{path, BldConfig};
use bld_core::database::new_connection_pool;
use bld_core::high_avail::HighAvail;
use bld_core::proxies::ServerPipelineProxy;
use bld_core::workers::PipelineWorker;
use bld_supervisor::base::{UnixSocketWrite, UnixSocketMessage};
use bld_supervisor::client::UnixSocketWriter;
use std::env::{set_var, temp_dir};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::info;

pub async fn start(config: BldConfig, host: &str, port: i64) -> anyhow::Result<()> {
    info!("starting bld server at {}:{}", host, port);
    let sock_path = path![temp_dir(), &config.local.unix_sock];
    let supervisor = web::Data::new(Mutex::new(UnixSocketWriter::connect(sock_path).await?));
    let pool = new_connection_pool(&config.local.db)?;
    let ha = web::Data::new(HighAvail::new(&config, pool.clone()).await?);
    let pool = web::Data::new(pool);
    let cfg = web::Data::new(config);
    let prx = web::Data::new(ServerPipelineProxy::new(
        Arc::clone(&cfg),
        Arc::clone(&pool),
    ));
    {
        let supervisor = supervisor.lock().unwrap();
        supervisor.try_write(&UnixSocketMessage::ServerAck).await
    }?;
    set_var("RUST_LOG", "actix_server=info,actix_web=debug");
    HttpServer::new(move || {
        App::new()
            .app_data(cfg.clone())
            .app_data(supervisor.clone())
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
