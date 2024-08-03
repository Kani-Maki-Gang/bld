use actix_web::{get, web::Data, HttpResponse, Responder};
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_models::{
    dtos::{CompletedPipelinesKpi, QueuedPipelinesKpi, RunningPipelinesKpi},
    pipeline_runs,
};
use sea_orm::DatabaseConnection;
use tracing::info;

async fn get_count_of_queued_pipelines(conn: &DatabaseConnection) -> Result<QueuedPipelinesKpi> {
    pipeline_runs::count_queued(conn)
        .await
        .map(|x| QueuedPipelinesKpi { count: x })
}

#[get("/v1/ui/kpis/queued-pipelines")]
pub async fn queued_pipelines(conn: Data<DatabaseConnection>) -> impl Responder {
    info!("Reached handler for /v1/ui/kpis/queued-pipelines route");
    match get_count_of_queued_pipelines(&conn).await {
        Ok(kpi) => HttpResponse::Ok().json(kpi),
        Err(e) => {
            info!("could not get the count of queued pipelines due to: {e}");
            HttpResponse::BadRequest().body("")
        }
    }
}

async fn get_count_of_running_pipelines(
    config: &BldConfig,
    conn: &DatabaseConnection,
) -> Result<RunningPipelinesKpi> {
    pipeline_runs::count_running(&conn)
        .await
        .and_then(|count| TryInto::<i64>::try_into(count).map_err(|e| anyhow!(e)))
        .map(|count| {
            let available_workers = config.local.supervisor.workers - count;
            RunningPipelinesKpi {
                count,
                available_workers,
            }
        })
}

#[get("/v1/ui/kpis/running-pipelines")]
pub async fn running_pipelines(
    config: Data<BldConfig>,
    conn: Data<DatabaseConnection>,
) -> impl Responder {
    info!("Reached handler for /v1/ui/kpis/running-pipelines route");
    match get_count_of_running_pipelines(&config, &conn).await {
        Ok(kpi) => HttpResponse::Ok().json(kpi),
        Err(e) => {
            info!("could not get the count of running pipelines due to: {e}");
            HttpResponse::BadRequest().body("")
        }
    }
}

async fn get_completed_pipelines(conn: &DatabaseConnection) -> Result<CompletedPipelinesKpi> {
    let count_per_state = pipeline_runs::count_per_state_last_ten_days(conn).await?;

    let finished_percentage = (count_per_state.finished as i64)
        .checked_div((count_per_state.finished + count_per_state.faulted) as i64)
        .map(|x| x as f64 * 100.0)
        .unwrap_or(0.0);

    let faulted_percentage = 100.0 - finished_percentage;

    Ok(CompletedPipelinesKpi {
        finished_count: count_per_state.finished,
        faulted_count: count_per_state.faulted,
        finished_percentage,
        faulted_percentage,
    })
}

#[get("/v1/ui/kpis/completed-pipelines")]
pub async fn completed_pipelines(conn: Data<DatabaseConnection>) -> impl Responder {
    info!("Reached handler for /v1/ui/kpis/completed-pipelines route");
    match get_completed_pipelines(&conn).await {
        Ok(kpi) => HttpResponse::Ok().json(kpi),
        Err(e) => {
            info!("could not get the count of completed pipelines due to: {e}");
            HttpResponse::BadRequest().body("")
        }
    }
}
