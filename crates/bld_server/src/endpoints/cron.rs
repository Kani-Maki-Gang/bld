use actix_web::{
    get, patch, post,
    web::{Data, Json, Path},
    HttpResponse, Responder, delete,
};
use anyhow::Result;
use bld_core::{requests::CronRequest, responses::CronJobResponse};
use tracing::info;

use crate::{
    cron::{CronScheduler, UpsertJob, RemoveJob},
    extractors::User,
};

#[get("/cron")]
pub async fn get(_: User, cron: Data<CronScheduler>) -> impl Responder {
    info!("Reached handler for GET /cron route");
    match do_get(cron.get_ref()) {
        Ok(res) => HttpResponse::Ok().json(res),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

fn do_get(cron: &CronScheduler) -> Result<Vec<CronJobResponse>> {
    cron.get().map(|jobs| {
        jobs.into_iter()
            .map(|j| CronJobResponse {
                id: j.id,
                schedule: j.schedule,
                pipeline: j.pipeline,
            })
            .collect()
    })
}

#[post("/cron")]
pub async fn post(_: User, cron: Data<CronScheduler>, body: Json<CronRequest>) -> impl Responder {
    info!("Reached handler for POST /cron route");
    match do_post(cron.get_ref(), body.into_inner()).await {
        Ok(_) => HttpResponse::Ok().body(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

async fn do_post(cron: &CronScheduler, data: CronRequest) -> Result<()> {
    let upsert_job = UpsertJob::new(
        data.schedule,
        data.pipeline,
        data.variables,
        data.environment,
    );
    cron.add(&upsert_job).await
}

#[patch("/cron")]
pub async fn patch(_: User, cron: Data<CronScheduler>, body: Json<CronRequest>) -> impl Responder {
    info!("Reached handler for PATCH /cron route");
    match do_patch(cron.get_ref(), body.into_inner()).await {
        Ok(_) => HttpResponse::Ok().body(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

async fn do_patch(cron: &CronScheduler, data: CronRequest) -> Result<()> {
    let upsert_job = UpsertJob::new(
        data.schedule,
        data.pipeline,
        data.variables,
        data.environment,
    );
    cron.upsert(&upsert_job).await
}

#[delete("/cron/{cron_job_id}")]
pub async fn delete(_: User, cron: Data<CronScheduler>, path: Path<String>) -> impl Responder {
    info!("Reached handler for DELETE /cron route");
    let cron_job_id = path.into_inner();
    match do_delete(cron.get_ref(), cron_job_id).await {
        Ok(_) => HttpResponse::Ok().body(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

async fn do_delete(cron: &CronScheduler, cron_job_id: String) -> Result<()> {
    let remove_job = RemoveJob::new(cron_job_id);
    cron.remove(&remove_job).await
}
