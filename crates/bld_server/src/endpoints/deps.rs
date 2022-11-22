use crate::extractors::User;
use actix_web::web::{Data, Json};
use actix_web::{post, HttpResponse, Responder};
use bld_core::proxies::PipelineFileSystemProxy;
use bld_runner::Pipeline;
use tracing::info;

#[post("/deps")]
pub async fn deps(
    _user: User,
    prx: Data<PipelineFileSystemProxy>,
    body: Json<String>,
) -> impl Responder {
    info!("Reached handler for /deps route");
    let name = body.into_inner();
    match Pipeline::dependencies(prx.get_ref(), &name) {
        Ok(r) => HttpResponse::Ok().json(r),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}
