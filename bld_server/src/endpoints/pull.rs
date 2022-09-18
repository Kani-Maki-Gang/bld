use crate::extractors::User;
use crate::responses::PullResponse;
use actix_web::{
    post,
    web::{Data, Json},
    HttpResponse, Responder,
};
use bld_core::proxies::{PipelineFileSystemProxy, ServerPipelineProxy};
use tracing::info;

#[post("/pull")]
pub async fn pull(
    user: Option<User>,
    prx: Data<ServerPipelineProxy>,
    body: Json<String>,
) -> impl Responder {
    info!("Reached handler for /pull route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
    let name = body.into_inner();
    match prx.read(&name) {
        Ok(r) => HttpResponse::Ok().json(PullResponse::new(&name, &r)),
        Err(_) => HttpResponse::BadRequest().body("Pipeline not found"),
    }
}
