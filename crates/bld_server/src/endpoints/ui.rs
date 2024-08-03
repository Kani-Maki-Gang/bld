use actix_web::{get, web::Data, HttpResponse, Responder};
use anyhow::Result;
use bld_models::{dtos::KpiInfo, pipeline_runs};
use chrono::{Duration, Utc};
use sea_orm::DatabaseConnection;
use tracing::info;

async fn get_count_of_queued_pipelines(conn: &DatabaseConnection) -> Result<KpiInfo> {
    let previous_date_time = (Utc::now() - Duration::days(10)).naive_utc();
    let previous_days_count =
        pipeline_runs::count_pipelines_runs_on_date(conn, previous_date_time).await?;

    let current_date_time = Utc::now().naive_utc();
    let current_days_count =
        pipeline_runs::count_pipelines_runs_on_date(conn, current_date_time).await?;

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
