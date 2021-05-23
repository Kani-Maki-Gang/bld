use crate::high_avail::AgentRequest;
use actix_web::{post, web, HttpResponse, Responder};
use async_raft::raft::AppendEntriesRequest;

#[post("/ha/appendEntries")]
pub async fn ha_append_entries(
    data: web::Json<AppendEntriesRequest<AgentRequest>>,
) -> impl Responder {
    let data = data.into_inner();
    dbg!(&data);
    HttpResponse::Ok().body("sending cool response")
}

#[post("/ha/installSnapshot")]
pub async fn ha_install_snapshot() -> impl Responder {
    HttpResponse::Ok().body("")
}

#[post("/ha/vote")]
pub async fn ha_vote() -> impl Responder {
    HttpResponse::Ok().body("")
}
