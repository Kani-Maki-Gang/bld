use crate::extractors::User;
use crate::supervisor::channel::SupervisorMessageSender;
use actix_web::web::{Data, Json};
use actix_web::{post, HttpResponse, Responder};
use tracing::info;

#[post("/v1/stop")]
pub async fn post(
    _: User,
    req: Json<String>,
    supervisor_sender: Data<SupervisorMessageSender>,
) -> impl Responder {
    info!("Reached handler for /stop route");
    match supervisor_sender.stop(&req).await {
        Ok(_) => HttpResponse::Ok().json(""),
        Err(_) => HttpResponse::BadRequest().body("pipeline not found"),
    }
}
