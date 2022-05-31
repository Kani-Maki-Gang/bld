use crate::config::BldConfig;
use crate::helpers::fs::IsYaml;
use crate::run::Pipeline;
use crate::persist::pipeline;
use crate::server::User;
use anyhow::anyhow;
use actix_web::{post, web, HttpResponse};
use diesel::r2d2::{ConnectionManager,Pool};
use diesel::sqlite::SqliteConnection;
use std::fs::remove_file;
use tracing::info;

#[post("/remove")]
pub async fn remove(
    user: Option<User>, 
    config: web::Data<BldConfig>,
    pool: web::Data<Pool<ConnectionManager<SqliteConnection>>>,
    body: web::Json<String>
) -> HttpResponse {
    info!("Reached handler for /remove route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
    match do_remove(config.get_ref(), pool.get_ref(), &body.into_inner()) {
        Ok(_) => HttpResponse::Ok().body(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string())
    }
}

fn do_remove(
    config: &BldConfig,
    pool: &Pool<ConnectionManager<SqliteConnection>>,
    name: &str
) -> anyhow::Result<()>
{
    let conn = pool.get()?;
    let pip = pipeline::select_by_name(&conn, name)?;
    let path = Pipeline::get_server_path(config, &pip.id)?;
    if path.is_yaml() {
        pipeline::delete(&conn, &pip.id)
            .map(|_| anyhow!("unable to remove pipeline"))?;
        let _ = remove_file(path);
        Ok(())
    } else {
        Err(anyhow!("pipeline not found"))
    }
}
