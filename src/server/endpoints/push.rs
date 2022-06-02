use crate::config::BldConfig;
use crate::persist::{pipeline, PipelineFileSystemProxy, ServerPipelineProxy};
use crate::push::PushInfo;
use crate::run::Pipeline;
use crate::server::User;
use actix_web::{post, web, HttpResponse, Responder};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use uuid::Uuid;
use std::sync::Arc;
use tracing::info;

#[post("/push")]
pub async fn push(
    user: Option<User>, 
    proxy: web::Data<ServerPipelineProxy>,
    pool: web::Data<Pool<ConnectionManager<SqliteConnection>>>,
    info: web::Json<PushInfo>
) -> impl Responder {
    info!("Reached handler for /push route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
    match do_push(proxy.get_ref(), pool.get_ref(), &info.into_inner()) {
        Ok(()) => HttpResponse::Ok().body(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

fn do_push(
    proxy: &impl PipelineFileSystemProxy,
    pool: &Pool<ConnectionManager<SqliteConnection>>,
    info: &PushInfo
) -> anyhow::Result<()> {
   let conn = pool.get()?; 
   if pipeline::select_by_name(&conn, &info.name).is_err() {
        let id = Uuid::new_v4().to_string();
        pipeline::insert(&conn, &id, &info.name)?;
   }
   proxy.create(&info.name, &info.content)
}
