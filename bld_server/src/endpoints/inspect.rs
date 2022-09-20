use crate::extractors::User;
use actix_web::{
    post,
    web::{Data, Json},
    HttpResponse, Responder,
};
use bld_core::proxies::PipelineFileSystemProxy;
use tracing::info;

#[post("/inspect")]
pub async fn inspect(
    user: Option<User>,
    prx: Data<PipelineFileSystemProxy>,
    body: Json<String>,
) -> impl Responder {
    info!("Reached handler for /inspect route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
    match prx.read(&body.into_inner()) {
        Ok(content) => HttpResponse::Ok().body(content),
        Err(_) => HttpResponse::BadRequest().body(""),
    }
}
