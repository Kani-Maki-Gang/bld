use crate::extractors::User;
use actix_web::{get, web::Data, web::Query, HttpResponse, Responder};
use anyhow::Result;
use bld_core::{
    database::{pipeline_runs, DbConnection},
    requests::HistQueryParams,
    responses::HistoryEntry,
};
use diesel::r2d2::{ConnectionManager, Pool};
use tracing::info;

#[get("/hist")]
pub async fn get(
    _user: User,
    db_pool: Data<Pool<ConnectionManager<DbConnection>>>,
    params: Query<HistQueryParams>,
) -> impl Responder {
    info!("Reached handler for /hist route");
    match history_info(db_pool.get_ref(), params.into_inner()) {
        Ok(ls) => HttpResponse::Ok().json(ls),
        Err(_) => HttpResponse::BadRequest().body(""),
    }
}

fn history_info(
    db_pool: &Pool<ConnectionManager<DbConnection>>,
    params: HistQueryParams,
) -> Result<Vec<HistoryEntry>> {
    let mut conn = db_pool.get()?;
    let history =
        pipeline_runs::select_with_filters(&mut conn, &params.state, &params.name, params.limit);
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
