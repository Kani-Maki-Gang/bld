use crate::config::definitions::{TOOL_DIR, TOOL_DEFAULT_CONFIG};
use actix_web::{get, HttpResponse};
use std::fs::{read_dir, DirEntry};

#[get("/list")]
pub async fn list() -> HttpResponse {
    if let Ok(dir) = read_dir(TOOL_DIR) {
        let pips = dir
            .filter(|e| e.is_ok())
            .map(|e| e.unwrap())
            .filter(|e| is_yaml_file(&e))
            .map(|e| get_file_name(&e))
            .fold(String::new(), |mut acc, n| {
                let line = format!("{}\n", n);
                acc.push_str(&line); 
                acc
            });
        return HttpResponse::Ok().body(pips);
    }
    HttpResponse::BadRequest().body("no pipelines found")
}

fn is_yaml_file(entry: &DirEntry) -> bool {
    match entry.file_type() {
        Ok(file_type) => {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            file_type.is_file()
            && name.ends_with(".yaml") 
            && name != format!("{}.yaml", TOOL_DEFAULT_CONFIG)
        }
        Err(_) => false
    }
}

fn get_file_name(entry: &DirEntry) -> String {
    let name = entry.file_name();
    let name = name.to_string_lossy();
    name[0..name.len() - 5].to_string()
}
