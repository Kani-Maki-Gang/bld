use crate::push::PushInfo;
use crate::run::Pipeline;
use crate::server::User;
use actix_web::{post, web, HttpResponse, Responder};
use std::fs::{create_dir_all, remove_file, File};
use std::io::Write;
use tracing::info;

#[post("/push")]
pub async fn push(user: Option<User>, info: web::Json<Vec<PushInfo>>) -> impl Responder {
    info!("Reached handler for /push route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
    match push_pipelines(info.into_inner()) {
        Ok(()) => HttpResponse::Ok().body(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

pub fn push_pipelines(info: Vec<PushInfo>) -> anyhow::Result<()> {
    for entry in info.iter() {
        let path = Pipeline::get_path(&entry.name)?;
        if path.is_file() {
            remove_file(&path)?;
        } else if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }
        let mut handle = File::create(&path)?;
        handle.write_all(entry.content.as_bytes())?;
    }
    Ok(())
}
