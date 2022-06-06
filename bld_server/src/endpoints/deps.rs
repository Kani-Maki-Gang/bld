use crate::extractors::User;
use actix_web::{post, web, HttpResponse, Responder};
use anyhow::anyhow;
use bld_config::BldConfig;
use bld_core::database::pipeline;
use bld_core::proxies::{PipelineFileSystemProxy, ServerPipelineProxy};
use bld_runner::Pipeline;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info};

#[post("/deps")]
pub async fn deps(
    user: Option<User>,
    prx: web::Data<ServerPipelineProxy>,
    body: web::Json<String>,
) -> impl Responder {
    info!("Reached handler for /deps route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
    let name = body.into_inner();
    match do_deps(prx.get_ref(), &name) {
        Ok(r) => HttpResponse::Ok().json(r),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

fn do_deps(prx: &impl PipelineFileSystemProxy, name: &str) -> anyhow::Result<Vec<String>> {
    deps_recursive(prx, name).map(|mut hs| {
        hs.remove(name);
        hs.into_iter().map(|(n, _)| n).collect()
    })
}

fn deps_recursive(
    prx: &impl PipelineFileSystemProxy,
    name: &str,
) -> anyhow::Result<HashMap<String, String>> {
    debug!("Parsing pipeline {name}");
    let src = prx
        .read(name)
        .map_err(|_| anyhow!("Pipeline with name: {name} not found"))?;
    let pipeline = Pipeline::parse(&src)?;
    let mut set = HashMap::new();
    set.insert(name.to_string(), src);
    for step in pipeline.steps.iter() {
        if let Some(pipeline) = &step.call {
            let subset = deps_recursive(prx, pipeline)?;
            for (k, v) in subset {
                set.insert(k, v);
            }
        }
    }
    Ok(set)
}