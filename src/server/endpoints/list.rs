use crate::config::BldConfig;
use crate::helpers::fs::IsYaml;
use crate::server::User;
use crate::persist::{pipeline, PipelineFileSystemProxy, ServerPipelineProxy};
use actix_web::{get, web, HttpResponse};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use tracing::info;
use std::sync::Arc;

#[get("/list")]
pub async fn list(
    user: Option<User>,
    proxy: web::Data<ServerPipelineProxy>,
    pool: web::Data<Pool<ConnectionManager<SqliteConnection>>>,
) -> HttpResponse {
    info!("Reached handler for /list route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
    match find_pipelines(proxy.get_ref(), pool.get_ref()) {
        Ok(pips) => HttpResponse::Ok().body(pips),
        Err(_) => HttpResponse::BadRequest().body("no pipelines found"),
    }
}

fn find_pipelines(proxy: &impl PipelineFileSystemProxy, pool: &Pool<ConnectionManager<SqliteConnection>>) -> anyhow::Result<String> {
    let conn = pool.get()?;
    let pips = pipeline::select_all(&conn)?
        .iter()
        .map(|p| (p, proxy.path(&p.name)))
        .filter(|(_, p)| p.is_ok())
        .filter(|(_, p)| p.as_ref().unwrap().is_yaml())
        .map(|(p, _)| p.name.clone())
        .fold(String::new(), |acc, n| format!("{acc}{n}\n"));
    Ok(pips)
}
