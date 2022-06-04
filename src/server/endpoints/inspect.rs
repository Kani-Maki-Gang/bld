use crate::config::BldConfig;
use crate::persist::{pipeline, PipelineFileSystemProxy, ServerPipelineProxy};
use crate::server::User;
use actix_web::{post, web, HttpResponse, Responder};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use std::sync::Arc;
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
