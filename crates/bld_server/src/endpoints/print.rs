use crate::extractors::User;
use actix_web::http::header;
use actix_web::web::{Data, Query, Header};
use actix_web::{get, HttpResponse, Responder};
use bld_core::fs::FileSystem;
use bld_models::dtos::PipelineQueryParams;
use bld_runner::{Yaml, Load};
use tracing::{debug, info};

#[get("/v1/print")]
pub async fn get(
    _: User,
    fs: Data<FileSystem>,
    params: Query<PipelineQueryParams>,
    accept: Header<header::Accept>
) -> impl Responder {
    info!("Reached handler for /print route");

    let Ok(pipeline) = fs.read(&params.pipeline).await else {
        return HttpResponse::BadRequest().body("pipeline not found");
    };

    let accept = accept.to_string();
    debug!("Accept: {accept}");

    if accept == "application/json" {
        return get_as_json(pipeline);
    }

    if accept == "text/plain" || accept == "*/*" || accept.is_empty() {
        return HttpResponse::Ok().body(pipeline);
    }

    HttpResponse::NotAcceptable().body("unsupported media type")
}

fn get_as_json(pipeline: String) -> HttpResponse {
    match Yaml::load(&pipeline) {
        Ok(pipeline) => HttpResponse::Ok().json(pipeline),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}
