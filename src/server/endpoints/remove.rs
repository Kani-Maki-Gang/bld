use crate::config::BldConfig;
use crate::helpers::fs::IsYaml;
use crate::persist::{pipeline, PipelineFileSystemProxy, ServerPipelineProxy};
use crate::server::User;
use actix_web::{post, web, HttpResponse};
use anyhow::anyhow;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use std::fs::remove_file;
use std::sync::Arc;
use tracing::info;

#[post("/remove")]
pub async fn remove(
    user: Option<User>,
    prx: web::Data<ServerPipelineProxy>,
    body: web::Json<String>,
) -> HttpResponse {
    info!("Reached handler for /remove route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
    match prx.remove(&body.into_inner()) {
        Ok(_) => HttpResponse::Ok().body(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}
