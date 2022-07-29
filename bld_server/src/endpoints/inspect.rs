use crate::extractors::User;
use actix_web::{post, web, HttpResponse, Responder};
use bld_core::proxies::{PipelineFileSystemProxy, ServerPipelineProxy};
use tracing::info;

#[post("/inspect")]
pub async fn inspect(
    user: Option<User>,
    prx: web::Data<ServerPipelineProxy>,
    body: web::Json<String>,
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
