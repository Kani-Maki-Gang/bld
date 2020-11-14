use crate::config::BldConfig;
use crate::server::{list_pipelines, ExecutePipelineSocket, MonitorPipelineSocket};
use crate::term::print_info;
use crate::types::Result;
use actix::{Arbiter, System};
use actix_web::{
    get, middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder,
};
use actix_web_actors::ws;
use clap::ArgMatches;

type StdResult<T, V> = std::result::Result<T, V>;

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Bld server running")
}

#[get("/list")]
async fn list() -> impl Responder {
    match list_pipelines() {
        Ok(ls) => HttpResponse::Ok().body(ls),
        Err(_) => HttpResponse::BadRequest().body(""),
    }
}

async fn ws_exec(req: HttpRequest, stream: web::Payload) -> StdResult<HttpResponse, Error> {
    println!("{:?}", req);
    let res = ws::start(ExecutePipelineSocket::new(), &req, stream);
    println!("{:?}", res);
    res
}

async fn ws_monit(req: HttpRequest, stream: web::Payload) -> StdResult<HttpResponse, Error> {
    println!("{:?}", req);
    let res = ws::start(MonitorPipelineSocket::new(), &req, stream);
    println!("{:?}", res);
    res
}

async fn start(host: &str, port: i64) -> Result<()> {
    print_info(&format!("starting bld server at {}:{}", host, port))?;
    std::env::set_var("RUST_LOG", "actix_server=info,actix_wev=info");
    env_logger::init();
    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .service(hello)
            .service(list)
            .service(web::resource("/ws-exec/").route(web::get().to(ws_exec)))
            .service(web::resource("/ws-monit").route(web::get().to(ws_monit)))
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await?;
    Ok(())
}

pub fn sys_spawn(host: String, port: i64) {
    let system = System::new("bld-server");
    Arbiter::spawn(async move {
        let _ = start(&host, port).await;
    });
    let _ = system.run();
}

pub fn exec(matches: &ArgMatches<'_>) -> Result<()> {
    let config = BldConfig::load()?;

    let host = match matches.value_of("host") {
        Some(host) => host.to_string(),
        None => config.local.host,
    };

    let port = match matches.value_of("port") {
        Some(port) => match port.parse::<i64>() {
            Ok(port) => port,
            Err(_) => config.local.port,
        },
        None => config.local.port,
    };

    sys_spawn(host, port);
    Ok(())
}
