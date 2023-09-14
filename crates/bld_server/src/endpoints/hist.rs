use crate::extractors::User;
use actix_web::{get, web::Data, web::Query, HttpResponse, Responder};
use anyhow::Result;
use bld_core::{database::pipeline_runs, requests::HistQueryParams, responses::HistoryEntry};
use sea_orm::DatabaseConnection;
use tracing::info;

#[get("/hist")]
pub async fn get(
    _user: User,
    conn: Data<DatabaseConnection>,
    params: Query<HistQueryParams>,
) -> impl Responder {
    info!("Reached handler for /hist route");
    match history_info(conn.get_ref(), params.into_inner()) {
        Ok(ls) => HttpResponse::Ok().json(ls),
        Err(_) => HttpResponse::BadRequest().body(""),
    }
}

fn history_info(conn: &DatabaseConnection, params: HistQueryParams) -> Result<Vec<HistoryEntry>> {
    let conn = conn.as_ref()?;
    let history =
        pipeline_runs::select_with_filters(&mut conn, &params.state, &params.name, params.limit)
            .await;
    let entries = history
        .map(|entries| {
            entries
                .into_iter()
                .map(|p| HistoryEntry {
                    name: p.name,
                    id: p.id,
                    user: p.app_user,
                    state: p.state,
                    start_date_time: p.start_date_time,
                    end_date_time: p.end_date_time.unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_else(|_| vec![]);
    Ok(entries)
}
