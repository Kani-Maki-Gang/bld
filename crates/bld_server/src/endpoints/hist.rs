use crate::extractors::User;
use actix_web::{HttpResponse, Responder, get, web::Data, web::Query};
use anyhow::Result;
use bld_models::{
    dtos::{HistQueryParams, HistoryEntry},
    pipeline_runs,
};
use sea_orm::DatabaseConnection;
use tracing::info;

#[get("/v1/hist")]
pub async fn get(
    _user: User,
    conn: Data<DatabaseConnection>,
    params: Query<HistQueryParams>,
) -> impl Responder {
    info!("Reached handler for /hist route");
    match history_info(conn.get_ref(), params.into_inner()).await {
        Ok(ls) => HttpResponse::Ok().json(ls),
        Err(_) => HttpResponse::BadRequest().body(""),
    }
}

async fn history_info(
    conn: &DatabaseConnection,
    params: HistQueryParams,
) -> Result<Vec<HistoryEntry>> {
    let history =
        pipeline_runs::select_with_filters(conn, &params.state, &params.name, params.limit).await;
    let entries = history
        .map(|entries| entries.into_iter().map(|p| p.into()).collect())
        .unwrap_or_else(|_| vec![]);
    Ok(entries)
}
