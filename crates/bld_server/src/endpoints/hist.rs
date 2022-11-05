use crate::extractors::User;
use crate::requests::HistQueryParams;
use crate::responses::HistoryEntry;
use actix_web::{get, web::Data, web::Query, HttpResponse, Responder};
use anyhow::Result;
use bld_core::database::pipeline_runs;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use tracing::info;

#[get("/hist")]
pub async fn hist(
    query: Query<HistQueryParams>,
    user: Option<User>,
    db_pool: Data<Pool<ConnectionManager<SqliteConnection>>>,
) -> impl Responder {
    info!("Reached handler for /hist route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
    let params = query.into_inner();
    match history_info(db_pool.get_ref(), &params.state, params.limit) {
        Ok(ls) => HttpResponse::Ok().json(ls),
        Err(_) => HttpResponse::BadRequest().body(""),
    }
}

fn history_info(
    db_pool: &Pool<ConnectionManager<SqliteConnection>>,
    state: &str,
    limit: i64,
) -> Result<Vec<HistoryEntry>> {
    let mut conn = db_pool.get()?;
    let history = pipeline_runs::select_with_filters(&mut conn, state, limit);
    let entries = history
        .map(|entries| {
            entries
                .into_iter()
                .map(|p| HistoryEntry {
                    name: p.name,
                    id: p.id,
                    user: p.user,
                    state: p.state,
                    start_date_time: p.start_date_time,
                    end_date_time: p.end_date_time.unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_else(|_| vec![]);
    Ok(entries)
}
