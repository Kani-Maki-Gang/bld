use crate::extractors::User;
use actix_web::web::{Data, Json};
use actix_web::{post, HttpResponse, Responder};
use bld_core::proxies::PipelineFileSystemProxy;
use tracing::info;

#[post("/inspect")]
pub async fn inspect(
    _: User,
    prx: Data<PipelineFileSystemProxy>,
    body: Json<String>,
) -> impl Responder {
    info!("Reached handler for /inspect route");
    match prx.read(&body.into_inner()) {
        Ok(content) => HttpResponse::Ok().json(content),
        Err(_) => HttpResponse::BadRequest().body(""),
    }
}
