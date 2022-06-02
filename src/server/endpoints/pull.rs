use crate::config::BldConfig;
use crate::persist::{pipeline, PipelineFileSystemProxy, ServerPipelineProxy};
use crate::pull::PullResponse;
use crate::server::User;
use actix_web::{post, web, HttpResponse, Responder};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use std::sync::Arc;
use tracing::info;

#[post("/pull")]
pub async fn pull(
    user: Option<User>,
    proxy: web::Data<ServerPipelineProxy>,
    body: web::Json<String>,
) -> impl Responder {
    info!("Reached handler for /pull route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
    let name = body.into_inner();
    match proxy.read(&name) {
        Ok(r) => HttpResponse::Ok().json(PullResponse::new(&name, &r)),
        Err(_) => HttpResponse::BadRequest().body("Pipeline not found"),
    }
}
