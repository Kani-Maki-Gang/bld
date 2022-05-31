use crate::config::BldConfig;
use crate::helpers::fs::IsYaml;
use crate::run::Pipeline;
use crate::server::User;
use crate::persist::pipeline;
use actix_web::{get, web, HttpResponse};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use tracing::info;

#[get("/list")]
pub async fn list(
    (user, db_pool, config): (
        Option<User>,
        web::Data<Pool<ConnectionManager<SqliteConnection>>>,
        web::Data<BldConfig>,
    )
) -> HttpResponse {
    info!("Reached handler for /list route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
    match find_pipelines(config.get_ref(), db_pool.get_ref()) {
        Ok(pips) => {
            let pips = pips.join("\n");
            HttpResponse::Ok().body(pips)
        }
        Err(_) => HttpResponse::BadRequest().body("no pipelines found") 
    }
}

fn find_pipelines(config: &BldConfig, pool: &Pool<ConnectionManager<SqliteConnection>>) -> anyhow::Result<Vec<String>> {
    let conn = pool.get()?;
    let pips = pipeline::select_all(&conn)?
        .iter()
        .map(|p| (p, Pipeline::get_server_path(config, &p.id)))
        .filter(|(_, p)| p.is_ok())
        .filter(|(_, p)| p.as_ref().unwrap().is_yaml())
        .map(|(p, _)| p.name.clone())
        .collect();
    Ok(pips)
}
