use actix_web::{
    HttpResponse, Responder, delete, get,
    http::header,
    web::{Data, Path, Query},
};
use bld_config::BldConfig;
use bld_models::{
    artifacts::{delete_by_id, select_by_id, select_by_run_id},
    dtos::{ArtifactResponse, ArtifactsQueryParams},
};
use sea_orm::DatabaseConnection;
use tracing::{info, warn};

use crate::extractors::User;

#[get("/v1/artifacts")]
pub async fn get(
    _: User,
    conn: Data<DatabaseConnection>,
    params: Query<ArtifactsQueryParams>,
) -> impl Responder {
    info!("Reached handler for GET /artifacts route");
    match select_by_run_id(conn.get_ref(), &params.run_id).await {
        Ok(artifacts) => {
            let response: Vec<ArtifactResponse> = artifacts.into_iter().map(Into::into).collect();
            HttpResponse::Ok().json(response)
        }
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

#[get("/v1/artifacts/{id}/download")]
pub async fn download(
    _: User,
    conn: Data<DatabaseConnection>,
    config: Data<BldConfig>,
    path: Path<String>,
) -> impl Responder {
    info!("Reached handler for GET /artifacts/{{id}}/download route");
    let id = path.into_inner();

    let artifact = match select_by_id(conn.get_ref(), &id).await {
        Ok(artifact) => artifact,
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
    };

    let artifact_path = config.artifact_full_path(&artifact.run_id, &artifact.id);
    match tokio::fs::read(&artifact_path).await {
        Ok(bytes) => HttpResponse::Ok()
            .content_type("application/gzip")
            .insert_header((
                header::CONTENT_DISPOSITION,
                format!(
                    "attachment; filename=\"{}.tar.gz\"",
                    escape_content_disposition_filename(&artifact.name)
                ),
            ))
            .body(bytes),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

fn escape_content_disposition_filename(name: &str) -> String {
    name.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::escape_content_disposition_filename;

    #[test]
    fn escape_content_disposition_filename_plain() {
        assert_eq!(
            escape_content_disposition_filename("my-artifact"),
            "my-artifact"
        );
    }

    #[test]
    fn escape_content_disposition_filename_escapes_quotes() {
        assert_eq!(
            escape_content_disposition_filename("foo\"bar"),
            "foo\\\"bar"
        );
    }

    #[test]
    fn escape_content_disposition_filename_escapes_backslashes() {
        assert_eq!(
            escape_content_disposition_filename("foo\\bar"),
            "foo\\\\bar"
        );
    }
}

#[delete("/v1/artifacts/{id}")]
pub async fn delete(
    _: User,
    conn: Data<DatabaseConnection>,
    config: Data<BldConfig>,
    path: Path<String>,
) -> impl Responder {
    info!("Reached handler for DELETE /artifacts route");
    let id = path.into_inner();

    let artifact = match select_by_id(conn.get_ref(), &id).await {
        Ok(artifact) => artifact,
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
    };

    if let Err(e) = delete_by_id(conn.get_ref(), &id).await {
        return HttpResponse::BadRequest().body(e.to_string());
    }

    let artifact_path = config.artifact_full_path(&artifact.run_id, &artifact.id);
    if let Err(e) = tokio::fs::remove_file(&artifact_path).await {
        warn!("unable to remove artifact file at {artifact_path:?}: {e}");
    }

    HttpResponse::Ok().json("")
}
