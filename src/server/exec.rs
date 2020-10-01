use crate::{config::BldConfig, term::print_info, server::RunPipelineWS};
use actix_web::{
    web, get, App, Error, HttpRequest, 
    HttpResponse, HttpServer, Responder
};
use actix_web_actors::ws;
use clap::ArgMatches;
use std::io;

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Bld server running")
}

async fn ws_pipeline(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    ws::start(RunPipelineWS, &req, stream)
}

async fn start(host: &str, port: i64) -> io::Result<()> {
    print_info(&format!("starting bld server at {}:{}", host, port))?;

    HttpServer::new(|| { 
            App::new()
                .service(hello)
                .service(web::resource("/ws/").route(web::get().to(ws_pipeline)))
        })
        .bind(format!("{}:{}", host, port))?
        .run()
        .await
}

pub async fn exec(matches: &ArgMatches<'_>) -> io::Result<()> {
    let config = BldConfig::load()?;

    let host = match matches.value_of("host") {
        Some(host) => host,
        None => &config.local.host,
    };

    let port = match matches.value_of("port") {
        Some(port) => match port.parse::<i64>() {
            Ok(port) => port,
            Err(_) => config.local.port,
        },
        None => config.local.port,
    };

    start(host, port).await
}
