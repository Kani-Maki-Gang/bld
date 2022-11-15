use crate::extractors::User;
use crate::responses::PullResponse;
use actix_web::web::{Data, Json};
use actix_web::{post, HttpResponse, Responder};
use bld_core::proxies::PipelineFileSystemProxy;
use tracing::info;

#[post("/pull")]
pub async fn pull(
    _: User,
    prx: Data<PipelineFileSystemProxy>,
    body: Json<String>,
) -> impl Responder {
    info!("Reached handler for /pull route");
    let name = body.into_inner();
    match prx.read(&name) {
        Ok(r) => HttpResponse::Ok().json(PullResponse::new(&name, &r)),
        Err(_) => HttpResponse::BadRequest().body("Pipeline not found"),
    }
}
