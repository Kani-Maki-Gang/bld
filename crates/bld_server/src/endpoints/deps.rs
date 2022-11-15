use crate::extractors::User;
use actix_web::web::{Data, Json};
use actix_web::{post, HttpResponse, Responder};
use anyhow::{anyhow, Result};
use bld_core::proxies::PipelineFileSystemProxy;
use bld_runner::Pipeline;
use std::collections::HashMap;
use tracing::{debug, info};

#[post("/deps")]
pub async fn deps(
    _user: User,
    prx: Data<PipelineFileSystemProxy>,
    body: Json<String>,
) -> impl Responder {
    info!("Reached handler for /deps route");
    let name = body.into_inner();
    match do_deps(prx.get_ref(), &name) {
        Ok(r) => HttpResponse::Ok().json(r),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

fn do_deps(prx: &PipelineFileSystemProxy, name: &str) -> Result<Vec<String>> {
    deps_recursive(prx, name).map(|mut hs| {
        hs.remove(name);
        hs.into_iter().map(|(n, _)| n).collect()
    })
}

fn deps_recursive(prx: &PipelineFileSystemProxy, name: &str) -> Result<HashMap<String, String>> {
    debug!("Parsing pipeline {name}");

    let src = prx
        .read(name)
        .map_err(|_| anyhow!("Pipeline with name: {name} not found"))?;
    let pipeline = Pipeline::parse(&src)?;

    let mut set = HashMap::new();
    set.insert(name.to_string(), src);

    for step in pipeline.steps.iter() {
        for call in &step.call {
            let subset = deps_recursive(prx, call)?;
            for (k, v) in subset {
                set.insert(k, v);
            }
        }
    }

    Ok(set)
}
