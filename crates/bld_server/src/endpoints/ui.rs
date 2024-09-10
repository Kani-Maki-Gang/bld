use actix_web::{get, web::Data, HttpResponse, Responder};
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_models::{
    dtos::{
        CompletedPipelinesKpi, PipelinePerCompletedStateKpi, PipelineRunsPerMonthKpi,
        QueuedPipelinesKpi, RunningPipelinesKpi, RunsPerUserKpi,
    },
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
    pipeline_runs::count_running(conn)
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
    let finished_count = count_per_state.finished as f64;
    let faulted_count = count_per_state.faulted as f64;
    let total_count = finished_count + faulted_count;
    let mut finished_percentage = 0.0;
    let mut faulted_percentage = 0.0;

    if total_count > 0.0 {
        finished_percentage = (finished_count / total_count) * 100.0;
        faulted_percentage = (faulted_count / total_count) * 100.0;
    }

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

async fn get_most_runs_per_user(conn: &DatabaseConnection) -> Result<Vec<RunsPerUserKpi>> {
    pipeline_runs::most_runs_per_user(conn).await.map(|x| {
        x.into_iter()
            .map(|u| RunsPerUserKpi {
                count: u.count,
                user: u.app_user,
            })
            .collect()
    })
}

#[get("/v1/ui/kpis/most-runs-per-user")]
pub async fn most_runs_per_user(conn: Data<DatabaseConnection>) -> impl Responder {
    info!("Reached handler for /v1/ui/kpis/most-runs-per-user route");
    match get_most_runs_per_user(&conn).await {
        Ok(kpi) => HttpResponse::Ok().json(kpi),
        Err(e) => {
            info!("could not get the count of most runs per user due to: {e}");
            HttpResponse::BadRequest().body("")
        }
    }
}

async fn get_pipelines_per_completed_state(
    conn: &DatabaseConnection,
) -> Result<Vec<PipelinePerCompletedStateKpi>> {
    pipeline_runs::select_per_completed_state(conn)
        .await
        .map(|x| {
            x.into_iter()
                .map(|p| {
                    let finished_count = p.finished_count as f64;
                    let faulted_count = p.faulted_count as f64;
                    let total_count = finished_count + faulted_count;
                    let mut finished_percentage = 0.0;
                    let mut faulted_percentage = 0.0;

                    if (finished_count + faulted_count) > 0.0 {
                        finished_percentage = (finished_count / total_count) * 100.0;
                        faulted_percentage = (faulted_count / total_count) * 100.0;
                    }

                    PipelinePerCompletedStateKpi {
                        pipeline: p.pipeline,
                        finished_percentage,
                        faulted_percentage,
                    }
                })
                .collect()
        })
}

#[get("/v1/ui/kpis/pipelines-per-completed-state")]
pub async fn pipelines_per_completed_state(conn: Data<DatabaseConnection>) -> impl Responder {
    info!("Reached handler for /v1/ui/kpis/pipelines-per-completed-state route");
    match get_pipelines_per_completed_state(&conn).await {
        Ok(kpi) => HttpResponse::Ok().json(kpi),
        Err(e) => {
            info!("could not get the count of pipelines per completed state due to: {e}");
            HttpResponse::BadRequest().body("")
        }
    }
}

async fn get_pipeline_runs_per_month(
    conn: &DatabaseConnection,
) -> Result<Vec<PipelineRunsPerMonthKpi>> {
    pipeline_runs::runs_per_month(conn).await.map(|x| {
        x.into_iter()
            .map(|p| PipelineRunsPerMonthKpi {
                month: p.month,
                count: p.count as f64,
            })
            .collect()
    })
}

#[get("/v1/ui/kpis/pipeline-runs-per-month")]
pub async fn pipeline_runs_per_month(conn: Data<DatabaseConnection>) -> impl Responder {
    info!("Reached handler for /v1/ui/kpis/pipeline-runs-per-month route");
    match get_pipeline_runs_per_month(&conn).await {
        Ok(kpi) => HttpResponse::Ok().json(kpi),
        Err(e) => {
            info!("could not get the count of pipeline runs per month due to: {e}");
            HttpResponse::BadRequest().body("")
        }
    }
}
