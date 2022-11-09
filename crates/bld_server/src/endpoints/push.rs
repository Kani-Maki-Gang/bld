use crate::extractors::User;
use crate::requests::PushInfo;
use actix_web::web::{Data, Json};
use actix_web::{post, HttpResponse, Responder};
use anyhow::Result;
use bld_core::database::pipeline;
use bld_core::proxies::PipelineFileSystemProxy;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use tracing::info;
use uuid::Uuid;

#[post("/push")]
pub async fn push(
    user: Option<User>,
    prx: Data<PipelineFileSystemProxy>,
    pool: Data<Pool<ConnectionManager<SqliteConnection>>>,
    info: Json<PushInfo>,
) -> impl Responder {
    info!("Reached handler for /push route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
    match do_push(prx.get_ref(), pool.get_ref(), &info.into_inner()) {
        Ok(()) => HttpResponse::Ok().body(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

fn do_push(
    prx: &PipelineFileSystemProxy,
    pool: &Pool<ConnectionManager<SqliteConnection>>,
    info: &PushInfo,
) -> Result<()> {
    let mut conn = pool.get()?;
    if pipeline::select_by_name(&mut conn, &info.name).is_err() {
        let id = Uuid::new_v4().to_string();
        pipeline::insert(&mut conn, &id, &info.name)?;
    }
    prx.create(&info.name, &info.content)
}
