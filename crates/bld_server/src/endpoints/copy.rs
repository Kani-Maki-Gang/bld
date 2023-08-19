use actix_web::{
    post,
    web::{Data, Json},
    HttpResponse, Responder,
};
use bld_core::{proxies::PipelineFileSystemProxy, requests::PipelinePathRequest};
use tracing::info;

use crate::extractors::User;

#[post("/copy")]
pub async fn post(
    _user: User,
    proxy: Data<PipelineFileSystemProxy>,
    body: Json<PipelinePathRequest>,
) -> impl Responder {
    info!("Reached handler for /copy route");
    match proxy.copy(&body.pipeline, &body.target) {
        Ok(_) => HttpResponse::Ok().body(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}
