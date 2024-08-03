use actix_web::{get, web::Path, HttpResponse, Responder};
use mime_guess::from_path;
use rust_embed::Embed;
use tracing::info;

#[derive(Embed)]
#[folder = "static_files/"]
struct StaticFiles;

fn get_static_file(path: &str) -> impl Responder {
    match StaticFiles::get(path) {
        Some(content) => HttpResponse::Ok()
            .content_type(from_path(path).first_or_octet_stream().as_ref())
            .body(content.data.into_owned()),

        None if from_path(path).is_empty() => get_static_file("index.html"),

        None => HttpResponse::NotFound().body("404 Not Found"),
    }
}

#[get("/")]
async fn index() -> impl Responder {
    get_static_file("index.html")
}

#[get("/{_:.*}")]
async fn fallback(path: Path<String>) -> impl Responder {
    info!("Reached handler for / route");
    get_static_file(&path)
}
