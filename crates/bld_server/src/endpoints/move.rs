use actix_web::{
    HttpResponse, Responder, patch,
    web::{Data, Json},
};
use bld_core::fs::FileSystem;
use bld_models::dtos::PipelinePathRequest;
use tracing::info;

use crate::extractors::User;

#[patch("/v1/move")]
pub async fn patch(
    _user: User,
    fs: Data<FileSystem>,
    body: Json<PipelinePathRequest>,
) -> impl Responder {
    info!("Reached handler for /move route");
    match fs.mv(&body.pipeline, &body.target).await {
        Ok(_) => HttpResponse::Ok().json(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}
