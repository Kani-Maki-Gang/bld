use actix_web::{
    HttpResponse, Responder, delete, get, patch, post,
    web::{Data, Json, Path, Query},
};
use bld_models::dtos::{AddJobRequest, JobFiltersParams, UpdateJobRequest};
use tracing::info;

use crate::{cron::CronScheduler, extractors::User};

#[get("/v1/cron")]
pub async fn get(
    _: User,
    cron: Data<CronScheduler>,
    query: Query<JobFiltersParams>,
) -> impl Responder {
    info!("Reached handler for GET /cron route");
    match cron.get(&query).await {
        Ok(res) => HttpResponse::Ok().json(res),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

#[post("/v1/cron")]
pub async fn post(_: User, cron: Data<CronScheduler>, body: Json<AddJobRequest>) -> impl Responder {
    info!("Reached handler for POST /cron route");
    match cron.add(&body).await {
        Ok(_) => HttpResponse::Ok().json(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

#[patch("/v1/cron")]
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

#[delete("/v1/cron/{cron_job_id}")]
pub async fn delete(_: User, cron: Data<CronScheduler>, path: Path<String>) -> impl Responder {
    info!("Reached handler for DELETE /cron route");
    let cron_job_id = path.into_inner();
    match cron.remove(&cron_job_id).await {
        Ok(_) => HttpResponse::Ok().json(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}
