use crate::config::BldConfig;
use crate::server::User;
use crate::persist::{pipeline, PipelineFileSystemProxy, ServerPipelineProxy};
use actix_web::{post, web, HttpResponse, Responder};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use std::sync::Arc;
use tracing::info;

#[post("/inspect")]
pub async fn inspect(
    user: Option<User>, 
    proxy: web::Data<ServerPipelineProxy>,
    pool: web::Data<Pool<ConnectionManager<SqliteConnection>>>,
    body: web::Json<String>
) -> impl Responder {
    info!("Reached handler for /inspect route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
    match do_inspect(proxy.get_ref(), pool.get_ref(), &body.into_inner()) {
        Ok(content) => HttpResponse::Ok().body(content),
        Err(_) => HttpResponse::BadRequest().body(""),
    }
}

fn do_inspect(
    proxy: &impl PipelineFileSystemProxy, 
    pool: &Pool<ConnectionManager<SqliteConnection>>, 
    name: &str
) -> anyhow::Result<String> {
    let conn = pool.get()?;
    let pipeline = pipeline::select_by_name(&conn, name)?;
    proxy.read(&pipeline.id)
}
