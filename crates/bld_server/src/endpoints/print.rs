use crate::extractors::User;
use actix_web::http::header;
use actix_web::web::{Data, Header, Query};
use actix_web::{get, HttpResponse, Responder};
use bld_core::fs::FileSystem;
use bld_models::dtos::PipelineInfoQueryParams;
use bld_runner::{Load, Yaml};
use tracing::{debug, info};

#[get("/v1/print")]
pub async fn get(
    _: User,
    fs: Data<FileSystem>,
    params: Query<PipelineInfoQueryParams>,
    accept: Header<header::Accept>,
) -> impl Responder {
    info!("Reached handler for /print route");

    let content = match params.into_inner() {
        PipelineInfoQueryParams::Id { id } => fs.read_by_id(&id).await,
        PipelineInfoQueryParams::Name { name } => fs.read(&name).await,
    };

    let Ok(content) = content else {
        return HttpResponse::BadRequest().body("pipeline not found");
    };

    let accept = accept.to_string();
    debug!("Accept: {accept}");

    if accept == "application/json" {
        return get_as_json(content);
    }

    if accept == "text/plain" || accept == "*/*" || accept.is_empty() {
        return HttpResponse::Ok().body(content);
    }

    HttpResponse::NotAcceptable().body("unsupported media type")
}

fn get_as_json(pipeline: String) -> HttpResponse {
    match Yaml::load(&pipeline) {
        Ok(pipeline) => HttpResponse::Ok().json(pipeline),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}
