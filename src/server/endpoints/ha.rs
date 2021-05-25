use crate::high_avail::{AgentRequest, HighAvail, HighAvailThreadRequest, HighAvailThreadResponse};
use actix_web::{post, web, HttpResponse, Responder};
use async_raft::raft::{AppendEntriesRequest, InstallSnapshotRequest, VoteRequest};
use uuid::Uuid;

#[post("/ha/appendEntries")]
pub async fn ha_append_entries(
    body: web::Json<AppendEntriesRequest<AgentRequest>>,
    ha: web::Data<HighAvail>,
) -> impl Responder {
    let body = body.into_inner();
    if let HighAvail::Enabled(th) = ha.get_ref() {
        let id = Uuid::new_v4();
        let send = {
            let req = HighAvailThreadRequest::AppendEntries(body);
            th.lock().unwrap().raft_request_tx().send((id, req))
        };
        if send.is_err() {
            return HttpResponse::BadRequest().body("unable to append entries");
        }
        while let Ok((resp_id, resp)) = th.lock().unwrap().raft_response_rx().recv() {
            if resp_id == id {
                return match resp {
                    Err(e) => HttpResponse::BadRequest().body(e.to_string()),
                    Ok(HighAvailThreadResponse::AppendEntries(r)) => HttpResponse::Ok().json(r),
                    Ok(_) => break,
                };
            }
        }
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
        let id = Uuid::new_v4();
        let send = {
            let req = HighAvailThreadRequest::InstallSnapshot(body);
            th.lock().unwrap().raft_request_tx().send((id, req))
        };
        if send.is_err() {
            return HttpResponse::BadRequest().body("unable to install snapshot");
        }
        while let Ok((resp_id, resp)) = th.lock().unwrap().raft_response_rx().recv() {
            if resp_id == id {
                return match resp {
                    Err(e) => HttpResponse::BadRequest().body(e.to_string()),
                    Ok(HighAvailThreadResponse::InstallSnapshot(r)) => HttpResponse::Ok().json(r),
                    Ok(_) => break,
                };
            }
        }
    }
    HttpResponse::BadRequest().body("server is not running is high availability mode")
}

#[post("/ha/vote")]
pub async fn ha_vote(body: web::Json<VoteRequest>, ha: web::Data<HighAvail>) -> impl Responder {
    let body = body.into_inner();
    if let HighAvail::Enabled(th) = ha.get_ref() {
        let id = Uuid::new_v4();
        let send = {
            let req = HighAvailThreadRequest::Vote(body);
            th.lock().unwrap().raft_request_tx().send((id, req))
        };
        if send.is_err() {
            return HttpResponse::BadRequest().body("unable to install snapshot");
        }
        while let Ok((resp_id, resp)) = th.lock().unwrap().raft_response_rx().recv() {
            if resp_id == id {
                return match resp {
                    Err(e) => HttpResponse::BadRequest().body(e.to_string()),
                    Ok(HighAvailThreadResponse::Vote(r)) => HttpResponse::Ok().json(r),
                    Ok(_) => break,
                };
            }
        }
    }
    HttpResponse::BadRequest().body("server is not running is high availability mode")
}
