use actix_web::{post, HttpResponse, Responder};
use actix_web::web::{Data, Json};
use async_raft::raft::{AppendEntriesRequest, InstallSnapshotRequest, VoteRequest};
use bld_core::high_avail::{AgentRequest, HighAvail};
use tracing::{debug, error, info};

#[post("/ha/appendEntries")]
pub async fn ha_append_entries(
    body: Json<AppendEntriesRequest<AgentRequest>>,
    ha: Data<HighAvail>,
) -> impl Responder {
    info!("Reached handler for /ha/appendEntries route");
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
    body: Json<InstallSnapshotRequest>,
    ha: Data<HighAvail>,
) -> impl Responder {
    info!("Reached handler for /ha/installSnapshot route");
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
pub async fn ha_vote(body: Json<VoteRequest>, ha: Data<HighAvail>) -> impl Responder {
    info!("Reached handler for /ha/vote route");
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
