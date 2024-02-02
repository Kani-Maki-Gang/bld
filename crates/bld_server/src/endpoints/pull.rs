use crate::extractors::User;
use actix_web::web::{Data, Query};
use actix_web::{get, HttpResponse, Responder};
use bld_core::proxies::PipelineFileSystemProxy;
use bld_dtos::{PipelineQueryParams, PullResponse};
use tracing::info;

#[get("/pull")]
pub async fn get(
    _: User,
    prx: Data<PipelineFileSystemProxy>,
    params: Query<PipelineQueryParams>,
) -> impl Responder {
    info!("Reached handler for /pull route");
    match prx.read(&params.pipeline).await {
        Ok(r) => HttpResponse::Ok().json(PullResponse::new(&params.pipeline, &r)),
        Err(_) => HttpResponse::BadRequest().body("Pipeline not found"),
    }
}
