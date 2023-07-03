use actix_web::{
    delete, get, patch, post,
    web::{Data, Json, Path, Query},
    HttpResponse, Responder,
};
use anyhow::Result;
use bld_core::{
    requests::{AddJobRequest, JobFiltersParams, UpdateJobRequest},
    responses::CronJobResponse,
};
use tracing::info;

use crate::{cron::CronScheduler, extractors::User};

#[get("/cron")]
pub async fn get(
    _: User,
    cron: Data<CronScheduler>,
    query: Query<JobFiltersParams>,
) -> impl Responder {
    info!("Reached handler for GET /cron route");
    match do_get(&cron, &query) {
        Ok(res) => HttpResponse::Ok().json(res),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

fn do_get(cron: &CronScheduler, filters: &JobFiltersParams) -> Result<Vec<CronJobResponse>> {
    cron.get(filters).map(|jobs| {
        jobs.into_iter()
            .map(|j| CronJobResponse {
                id: j.id,
                schedule: j.schedule,
                pipeline: j.pipeline,
                variables: j.variables,
                environment: j.environment,
                is_default: j.is_default,
            })
            .collect()
    })
}

#[post("/cron")]
pub async fn post(_: User, cron: Data<CronScheduler>, body: Json<AddJobRequest>) -> impl Responder {
    info!("Reached handler for POST /cron route");
    match cron.add(&body).await {
        Ok(_) => HttpResponse::Ok().json(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

#[patch("/cron")]
pub async fn patch(
    _: User,
    cron: Data<CronScheduler>,
    body: Json<UpdateJobRequest>,
) -> impl Responder {
    info!("Reached handler for PATCH /cron route");
    match cron.update(&body).await {
        Ok(_) => HttpResponse::Ok().json(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

#[delete("/cron/{cron_job_id}")]
pub async fn delete(_: User, cron: Data<CronScheduler>, path: Path<String>) -> impl Responder {
    info!("Reached handler for DELETE /cron route");
    let cron_job_id = path.into_inner();
    match cron.remove(&cron_job_id).await {
        Ok(_) => HttpResponse::Ok().json(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}
