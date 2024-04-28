use crate::extractors::User;
use actix_web::{
    get,
    http::header,
    web::{Data, Header},
    HttpResponse,
};
use bld_models::{dtos::ListResponse, pipeline};
use sea_orm::DatabaseConnection;
use tracing::info;

#[get("/v1/list")]
pub async fn get(
    _: User,
    conn: Data<DatabaseConnection>,
    accept: Header<header::Accept>,
) -> HttpResponse {
    info!("Reached handler for /list route");

    let Ok(pips) = pipeline::select_all(conn.as_ref()).await else {
        return HttpResponse::BadRequest().body("no pipelines found");
    };

    let accept = accept.to_string();

    if accept == "application/json" {
        let pips: Vec<ListResponse> = pips
            .into_iter()
            .map(|x| ListResponse {
                id: x.id,
                pipeline: x.name,
            })
            .collect();
        return HttpResponse::Ok().json(pips);
    }

    if accept == "text/plain" || accept == "*/*" || accept.is_empty() {
        let pips: Vec<String> = pips.into_iter().map(|x| x.name).collect();
        return HttpResponse::Ok().body(pips.join("\n"));
    }

    HttpResponse::BadRequest().body("unsupported media type")
}
