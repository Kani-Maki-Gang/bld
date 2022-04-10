use crate::config::definitions::{TOOL_DEFAULT_CONFIG, TOOL_DIR};
use crate::server::User;
use actix_web::{get, HttpResponse};
use std::fs::{read_dir, DirEntry};
use tracing::info;

#[get("/list")]
pub async fn list(user: Option<User>) -> HttpResponse {
    info!("Reached handler for /list route");
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }
    let mut pips = Vec::<String>::new();
    if find_pipelines(&mut pips, TOOL_DIR).is_ok() {
        let pips = pips.join("\n");
        return HttpResponse::Ok().body(pips);
    }
    HttpResponse::BadRequest().body("no pipelines found")
}

fn find_pipelines(collection: &mut Vec<String>, path: &str) -> anyhow::Result<()> {
    let rd = read_dir(path)?;
    for entry in rd {
        if entry.is_err() {
            continue;
        }
        let entry = entry.unwrap();
        let entry_path = entry.path();
        if entry_path.is_file() && is_yaml_file(&entry) {
            collection.push(entry_path.as_path().display().to_string());
        }
        if entry_path.is_dir() {
            if let Some(sub_dir) = entry_path.as_path().to_str() {
                find_pipelines(collection, sub_dir)?;
            }
        }
    }
    Ok(())
}

fn is_yaml_file(entry: &DirEntry) -> bool {
    match entry.file_type() {
        Ok(file_type) => {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            file_type.is_file()
                && name.ends_with(".yaml")
                && name != format!("{TOOL_DEFAULT_CONFIG}.yaml")
        }
        Err(_) => false,
    }
}
