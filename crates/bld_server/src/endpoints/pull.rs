use crate::extractors::User;
use actix_web::web::{Data, Query};
use actix_web::{HttpResponse, Responder, get};
use bld_core::fs::FileSystem;
use bld_models::dtos::{PipelineQueryParams, PullResponse};
use tracing::info;

#[get("/v1/pull")]
pub async fn get(
    _: User,
    fs: Data<FileSystem>,
    params: Query<PipelineQueryParams>,
) -> impl Responder {
    info!("Reached handler for /pull route");
    match fs.read(&params.pipeline).await {
        Ok(r) => HttpResponse::Ok().json(PullResponse::new(&params.pipeline, &r)),
        Err(_) => HttpResponse::BadRequest().body("File not found"),
    }
}
