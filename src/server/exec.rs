use crate::config::BldConfig;
use crate::server::PipelineWebSocketServer;
use crate::term::print_info;
use actix::{Arbiter, System};
use actix_web::{
    get, middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder,
};
use actix_web_actors::ws;
use clap::ArgMatches;
use std::io;

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Bld server running")
}

async fn ws_pipeline(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    println!("{:?}", req);
    let res = ws::start(PipelineWebSocketServer::new(), &req, stream);
    println!("{:?}", res);
    res
}

async fn start(host: &str, port: i64) -> io::Result<()> {
    print_info(&format!("starting bld server at {}:{}", host, port))?;
    std::env::set_var("RUST_LOG", "actix_server=info,actix_wev=info");
    env_logger::init();
    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .service(hello)
            .service(web::resource("/ws/").route(web::get().to(ws_pipeline)))
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await
}

pub fn exec(matches: &ArgMatches<'_>) -> io::Result<()> {
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

    let system = System::new("bld-server");

    Arbiter::spawn(async move {
        let _ = start(&host, port).await;
    });

    let _ = system.run();

    Ok(())
}
