use crate::pull::PullResponse;
use crate::run::Pipeline;
use crate::server::User;
use anyhow::anyhow;
use actix_web::{post, web, HttpResponse, Responder};
use std::collections::HashSet;
use tracing::{debug, info};

#[post("/deps")]
pub async fn deps(user: Option<User>, body: web::Json<String>) -> impl Responder {
    info!("Reached handler for /deps route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
    let name = body.into_inner();
    match build_payload(&name)
        .map(|mut hs| {
            hs.remove(&name);
            hs.into_iter().collect::<Vec<String>>()
        })
    {
        Ok(r) => HttpResponse::Ok().json(r),
        Err(e) => HttpResponse::BadRequest().body(e.to_string())
    }
}

#[post("/pull")]
pub async fn pull(user: Option<User>, body: web::Json<String>) -> impl Responder {
    info!("Reached handler for /pull route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
    let name = body.into_inner();
    match Pipeline::read(&name) {
        Ok(r) => HttpResponse::Ok().json(PullResponse::new(&name, &r)),
        Err(_) => HttpResponse::BadRequest().body("Pipeline not found"),
    }
}

pub fn build_payload(name: &str) -> anyhow::Result<HashSet<String>> {
    debug!("Parsing pipeline {name}");
    let src = Pipeline::read(name).map_err(|_| anyhow!("Pipeline not found"))?;
    let pipeline = Pipeline::parse(&src)?;
    let mut set = HashSet::new();
    set.insert(name.to_string());
    for step in pipeline.steps.iter() {
        if let Some(pipeline) = &step.call {
            let subset = build_payload(pipeline)?;
            for entry in subset {
                set.insert(entry);
            }
        }
    }
    Ok(set)
}
