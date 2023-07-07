use crate::extractors::User;
use actix_web::web::{Data, Query};
use actix_web::{get, HttpResponse, Responder};
use bld_core::proxies::PipelineFileSystemProxy;
use bld_core::requests::PrintQueryParams;
use tracing::info;

#[get("/print")]
pub async fn print(
    _: User,
    prx: Data<PipelineFileSystemProxy>,
    params: Query<PrintQueryParams>,
) -> impl Responder {
    info!("Reached handler for /print route");
    match prx.read(&params.pipeline) {
        Ok(content) => HttpResponse::Ok().json(content),
        Err(_) => HttpResponse::BadRequest().body("pipeline not found"),
    }
}
