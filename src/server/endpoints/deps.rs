use crate::config::BldConfig;
use crate::run::Pipeline;
use crate::server::User;
use crate::persist::pipeline;
use anyhow::anyhow;
use actix_web::{post, web, HttpResponse, Responder};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use std::collections::HashMap;
use tracing::{debug, info};

#[post("/deps")]
pub async fn deps(
    user: Option<User>, 
    config: web::Data<BldConfig>,
    pool: web::Data<Pool<ConnectionManager<SqliteConnection>>>,
    body: web::Json<String>
) -> impl Responder {
    info!("Reached handler for /deps route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
    let name = body.into_inner();
    match do_deps(config.get_ref(), pool.get_ref(), &name) {
        Ok(r) => HttpResponse::Ok().json(r),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

fn do_deps(
    config: &BldConfig, 
    pool: &Pool<ConnectionManager<SqliteConnection>>, 
    name: &str
) -> anyhow::Result<Vec<String>> {
    let conn = pool.get()?;
    deps_recursive(config, &conn, name).map(|mut hs| {
        hs.remove(name);
        hs.into_iter().map(|(n, _)| n).collect()
    })
}

fn deps_recursive(config: &BldConfig, conn: &SqliteConnection, name: &str) -> anyhow::Result<HashMap<String, String>> {
    debug!("Parsing pipeline {name}");
    let pip = pipeline::select_by_name(conn, name)?;
    let src = Pipeline::read_in_server(config, &pip.id).map_err(|_| anyhow!("Pipeline with id: {} and name: {name} not found", pip.id))?;
    let pipeline = Pipeline::parse(&src)?;
    let mut set = HashMap::new();
    set.insert(name.to_string(), src);
    for step in pipeline.steps.iter() {
        if let Some(pipeline) = &step.call {
            let subset = deps_recursive(config, conn, pipeline)?;
            for (k, v) in subset {
                set.insert(k, v);
            }
        }
    }
    Ok(set)
}
