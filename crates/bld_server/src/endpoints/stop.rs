use crate::extractors::User;
use crate::supervisor::channel::SupervisorMessageSender;
use actix_web::web::{Data, Json};
use actix_web::{post, HttpResponse, Responder};
use tracing::info;

#[post("/stop")]
pub async fn stop(
    _: User,
    req: Json<String>,
    supervisor_sender: Data<SupervisorMessageSender>
) -> impl Responder {
    info!("Reached handler for /stop route");
    let id = req.into_inner();
    match supervisor_sender.stop(&id).await {
        Ok(_) => HttpResponse::Ok().json(""),
        Err(_) => HttpResponse::BadRequest().body("pipeline not found"),
    }
}
