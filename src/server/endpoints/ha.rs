use crate::high_avail::{AgentRequest, HighAvail};
use actix_web::{post, web, HttpResponse, Responder};
use async_raft::raft::{AppendEntriesRequest, InstallSnapshotRequest, VoteRequest};
use tracing::{debug, error};

#[post("/ha/appendEntries")]
pub async fn ha_append_entries(
    body: web::Json<AppendEntriesRequest<AgentRequest>>,
    ha: web::Data<HighAvail>,
) -> impl Responder {
    let body = body.into_inner();
    if let HighAvail::Enabled(th) = ha.get_ref() {
        let raft = th.raft();
        return match raft.append_entries(body).await {
            Ok(r) => HttpResponse::Ok().json(r),
            Err(e) => HttpResponse::BadRequest().body(e.to_string()),
        };
    }
    HttpResponse::BadRequest().body("server is not running is high availability mode")
}

#[post("/ha/installSnapshot")]
pub async fn ha_install_snapshot(
    body: web::Json<InstallSnapshotRequest>,
    ha: web::Data<HighAvail>,
) -> impl Responder {
    let body = body.into_inner();
    if let HighAvail::Enabled(th) = ha.get_ref() {
        return match th.raft().install_snapshot(body).await {
            Ok(r) => HttpResponse::Ok().json(r),
            Err(e) => HttpResponse::BadRequest().body(e.to_string()),
        };
    }
    HttpResponse::BadRequest().body("server is not running is high availability mode")
}

#[post("/ha/vote")]
pub async fn ha_vote(body: web::Json<VoteRequest>, ha: web::Data<HighAvail>) -> impl Responder {
    let body = body.into_inner();
    if let HighAvail::Enabled(th) = ha.get_ref() {
        return match th.raft().vote(body).await {
            Ok(r) => {
                debug!("vote with response: {:?}", r);
                HttpResponse::Ok().json(r)
            }
            Err(e) => {
                error!("vote with error: {}", e);
                HttpResponse::BadRequest().body(e.to_string())
            }
        };
    }
    HttpResponse::BadRequest().body("server is not running is high availability mode")
}
