use crate::endpoints::{
    auth_redirect, deps, ha_append_entries, ha_install_snapshot, ha_vote, hist, home, inspect,
    list, pull, push, remove, run, stop,
};
use crate::sockets::{ws_exec, ws_high_avail, ws_monit};
use actix::io::SinkWrite;
use actix::{Actor, Addr, StreamHandler};
use actix_web::rt::spawn;
use actix_web::web::{get, resource, Data};
use actix_web::{middleware, App, HttpServer};
use anyhow::{anyhow, Result};
use awc::http::Version;
use awc::Client;
use bld_config::BldConfig;
use bld_core::database::new_connection_pool;
use bld_core::high_avail::HighAvail;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_sock::clients::EnqueueClient;
use bld_sock::messages::ServerMessages;
use futures::{join, stream::StreamExt};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use std::env::{current_exe, set_var};
use std::sync::Arc;
use tokio::process::{Child, Command};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tracing::{debug, error, info};

async fn spawn_server(
    config: Data<BldConfig>,
    host: String,
    port: i64,
    enqueue_tx: Sender<ServerMessages>,
) -> Result<()> {
    info!("starting bld server at {}:{}", host, port);

    let config_clone = config.clone();
    let pool = new_connection_pool(&config.local.db)?;
    let enqueue_tx = Data::new(enqueue_tx);
    let ha = Data::new(HighAvail::new(&config, pool.clone()).await?);
    let pool = Data::new(pool);
    let prx = Data::new(PipelineFileSystemProxy::Server {
        config: Arc::clone(&config),
        pool: Arc::clone(&pool),
    });

    set_var("RUST_LOG", "actix_server=info,actix_web=debug");
    let mut server = HttpServer::new(move || {
        App::new()
            .app_data(config_clone.clone())
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
            .service(run)
            .service(push)
            .service(deps)
            .service(pull)
            .service(stop)
            .service(inspect)
            .service(resource("/ws-exec/").route(get().to(ws_exec)))
            .service(resource("/ws-monit/").route(get().to(ws_monit)))
            .service(resource("/ws-ha/").route(get().to(ws_high_avail)))
    });

    let address = format!("{host}:{port}");
    server = match &config.local.server.tls {
        Some(tls) => {
            let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls())?;
            builder.set_private_key_file(&tls.private_key, SslFiletype::PEM)?;
            builder.set_certificate_chain_file(&tls.cert_chain)?;
            server.bind_openssl(address, builder)?
        }
        None => server.bind(address)?,
    };

    server.run().await?;
    Ok(())
}

async fn supervisor_socket(
    config: Arc<BldConfig>,
    mut enqueue_rx: Receiver<ServerMessages>,
) -> Result<Addr<EnqueueClient>> {
    let supervisor = &config.local.supervisor;
    let url = format!(
        "{}://{}:{}/ws-server/",
        supervisor.ws_protocol(),
        supervisor.host,
        supervisor.port
    );

    debug!("establishing web socket connection on {}", url);

    let client = Client::builder()
        .max_http_version(Version::HTTP_11)
        .finish();
    let (_, framed) = client.ws(url).connect().await.map_err(|e| {
        error!("{e}");
        anyhow!(e.to_string())
    })?;

    let (sink, stream) = framed.split();
    let addr = EnqueueClient::create(|ctx| {
        EnqueueClient::add_stream(stream, ctx);
        EnqueueClient::new(SinkWrite::new(sink, ctx))
    });

    addr.send(ServerMessages::Ack).await?;

    while let Some(msg) = enqueue_rx.recv().await {
        addr.send(msg).await?;
    }

    Ok(addr)
}

fn create_supervisor() -> Result<Child> {
    Ok(Command::new(current_exe()?).arg("supervisor").spawn()?)
}

pub async fn start(config: BldConfig, host: String, port: i64) -> Result<()> {
    let config = Data::new(config);
    let config_clone = Arc::clone(&config);
    let mut supervisor = create_supervisor()?; // set to kill the supervisor process on drop.
    let (enqueue_tx, enqueue_rx) = channel(4096);

    let web_server_handle = spawn(async move {
        if let Err(e) = spawn_server(config, host, port, enqueue_tx).await {
            error!("web server error, {e}");
        }
    });

    let socket_handle = spawn(async move {
        if let Err(e) = supervisor_socket(config_clone, enqueue_rx).await {
            error!("supervisor socket error, {e}");
        }
    });

    let result = join!(web_server_handle, socket_handle);

    if let Err(e) = result.0 {
        error!("{e}");
    }

    if let Err(e) = result.1 {
        error!("{e}");
    }

    supervisor.kill().await?;
    Ok(())
}
