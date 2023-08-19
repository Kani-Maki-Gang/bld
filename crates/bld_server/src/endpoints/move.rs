use actix_web::{
    patch,
    web::{Data, Json},
    HttpResponse, Responder,
};
use bld_core::{proxies::PipelineFileSystemProxy, requests::PipelinePathRequest};
use tracing::info;

use crate::extractors::User;

#[patch("/move")]
pub async fn patch(
    _user: User,
    proxy: Data<PipelineFileSystemProxy>,
    body: Json<PipelinePathRequest>,
) -> impl Responder {
    info!("Reached handler for /move route");
    match proxy.mv(&body.pipeline, &body.target) {
        Ok(_) => HttpResponse::Ok().body(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}
