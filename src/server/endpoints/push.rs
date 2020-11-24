use crate::run::Pipeline;
use crate::types::{PushInfo, Result};
use actix_web::{post, web, Responder, HttpResponse};
use std::fs::{remove_file, File};
use std::io::Write;

#[post("/push")]
pub async fn push(info: web::Json<Vec<PushInfo>>) -> impl Responder {
    match push_pipelines(info.into_inner()) {
        Ok(()) => HttpResponse::Ok().body(&String::new()),
        Err(e) => HttpResponse::BadRequest().body(&e.to_string()),
    }
}

pub fn push_pipelines(info: Vec<PushInfo>) -> Result<()> {
    for entry in info.iter() {
        let path = Pipeline::get_path(&entry.name)?;
        if path.is_file() {
            remove_file(&path)?;
        }
        let mut handle = File::create(&path)?;
        handle.write_all(entry.content.as_bytes())?;
    }
    Ok(())
}
