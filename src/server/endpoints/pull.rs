use crate::pull::{PullRequestInfo, PullResponseInfo};
use crate::run::Pipeline;
use crate::server::User;
use actix_web::{post, web, HttpResponse, Responder};
use std::collections::HashMap;
use tracing::{debug, info};

#[post("/pull")]
pub async fn pull(user: Option<User>, body: web::Json<PullRequestInfo>) -> impl Responder {
    info!("Reached handler for /pull route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
    let req = body.into_inner();
    let res: anyhow::Result<Vec<PullResponseInfo>> = build_payload(&req.name, req.include_deps)
        .map(|hmap| {
            hmap.iter()
                .map(|(k, v)| PullResponseInfo::new(k, v))
                .collect()
        });
    match res {
        Ok(r) => HttpResponse::Ok().json(r),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

pub fn build_payload(name: &str, include_deps: bool) -> anyhow::Result<HashMap<String, String>> {
    debug!("Parsing pipeline {name}");
    let src = Pipeline::read(name)?;
    let pipeline = Pipeline::parse(&src)?;
    let mut set = HashMap::new();
    set.insert(name.to_string(), src);
    if include_deps {
        for step in pipeline.steps.iter() {
            if let Some(pipeline) = &step.call {
                let subset = build_payload(pipeline, include_deps)?;
                for (k, v) in subset {
                    set.insert(k, v);
                }
            }
        }
    }
    Ok(set)
}
