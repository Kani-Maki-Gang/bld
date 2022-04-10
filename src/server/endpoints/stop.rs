use crate::server::{PipelinePool, User};
use actix_web::{post, web, HttpResponse};
use tracing::info;

#[post("/stop")]
pub fn stop(
    (user, req, data): (Option<User>, web::Json<String>, web::Data<PipelinePool>),
) -> HttpResponse {
    info!("Reached handler for /stop route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }

    let id = req.into_inner();
    let pool = data.senders.lock().unwrap();
    match pool.get(&id) {
        Some(sender) => match sender.send(true) {
            Ok(_) => HttpResponse::Ok().finish(),
            Err(e) => HttpResponse::BadRequest().body(e.to_string()),
        },
        None => {
            let message = format!("no pipeline with id {id} found");
            HttpResponse::BadRequest().body(message)
        }
    }
}
