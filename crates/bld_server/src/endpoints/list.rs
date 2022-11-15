use crate::extractors::User;
use actix_web::{get, web::Data, HttpResponse};
use anyhow::Result;
use bld_core::database::pipeline;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_utils::fs::IsYaml;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use tracing::info;

#[get("/list")]
pub async fn list(
    _: Option<User>,
    prx: Data<PipelineFileSystemProxy>,
    pool: Data<Pool<ConnectionManager<SqliteConnection>>>,
) -> HttpResponse {
    info!("Reached handler for /list route");
    match find_pipelines(prx.get_ref(), pool.get_ref()) {
        Ok(pips) => HttpResponse::Ok().json(pips),
        Err(_) => HttpResponse::BadRequest().body("no pipelines found"),
    }
}

fn find_pipelines(
    prx: &PipelineFileSystemProxy,
    pool: &Pool<ConnectionManager<SqliteConnection>>,
) -> Result<String> {
    let mut conn = pool.get()?;
    let pips = pipeline::select_all(&mut conn)?
        .iter()
        .map(|p| (p, prx.path(&p.name)))
        .filter(|(_, p)| p.as_ref().map(|p| p.is_yaml()).unwrap_or_default())
        .map(|(p, _)| p.name.clone())
        .fold(String::new(), |acc, n| format!("{acc}{n}\n"));
    Ok(pips)
}
