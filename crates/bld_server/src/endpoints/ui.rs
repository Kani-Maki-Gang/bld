use actix_web::{get, web::Data, HttpResponse, Responder};
use anyhow::Result;
use bld_config::BldConfig;
use bld_models::{dtos::KpiInfo, pipeline_runs};
use chrono::{Duration, Utc};
use sea_orm::DatabaseConnection;
use tracing::info;

async fn get_count_of_queued_pipelines(conn: &DatabaseConnection) -> Result<KpiInfo> {
    let previous_date_time = (Utc::now() - Duration::days(10)).naive_utc();
    let previous_days_count = pipeline_runs::count_queued_on_date(conn, previous_date_time).await?;

    let current_date_time = Utc::now().naive_utc();
    let current_days_count = pipeline_runs::count_queued_on_date(conn, current_date_time).await?;

    let count = current_days_count - previous_days_count;

    let percentage = (current_days_count as i32)
        .checked_div(previous_days_count as i32)
        .map(|x| x as f64 * 100.0)
        .unwrap_or(0.0);

    Ok(KpiInfo::new(count, percentage))
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
) -> Result<KpiInfo> {
    pipeline_runs::count_running(&conn).await.map(|count| {
        let percentage = config.local.supervisor.workers as f64 - count as f64;
        KpiInfo::new(count, percentage)
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
