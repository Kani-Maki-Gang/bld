use crate::extractors::User;
use actix_web::web::{Data, Query};
use actix_web::{get, HttpResponse, Responder};
use bld_core::fs::FileSystem;
use bld_models::dtos::PipelineQueryParams;
use tracing::info;

#[get("/print")]
pub async fn get(
    _: User,
    fs: Data<FileSystem>,
    params: Query<PipelineQueryParams>,
) -> impl Responder {
    info!("Reached handler for /print route");
    match fs.read(&params.pipeline).await {
        Ok(content) => HttpResponse::Ok().json(content),
        Err(_) => HttpResponse::BadRequest().body("pipeline not found"),
    }
}
