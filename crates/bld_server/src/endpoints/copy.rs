use actix_web::{
    post,
    web::{Data, Json},
    HttpResponse, Responder,
};
use bld_core::fs::FileSystem;
use bld_models::dtos::PipelinePathRequest;
use tracing::info;

use crate::extractors::User;

#[post("/v1/copy")]
pub async fn post(
    _user: User,
    fs: Data<FileSystem>,
    body: Json<PipelinePathRequest>,
) -> impl Responder {
    info!("Reached handler for /copy route");
    match fs.copy(&body.pipeline, &body.target).await {
        Ok(_) => HttpResponse::Ok().json(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}
