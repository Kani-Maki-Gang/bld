use actix_web::{
    HttpResponse, Responder,
    body::SizedStream,
    delete,
    error::ErrorInternalServerError,
    get,
    http::header,
    web::{Bytes, Data, Path, Query},
};
use bld_config::BldConfig;
use bld_models::{
    artifacts::{delete_by_id, select_by_id, select_by_run_id},
    dtos::{ArtifactResponse, ArtifactsQueryParams},
};
use futures::{Stream, stream::unfold};
use sea_orm::DatabaseConnection;
use tokio::{fs::File, io::AsyncReadExt};
use tracing::{info, warn};

use crate::extractors::User;

const DOWNLOAD_CHUNK_SIZE: usize = 64 * 1024;

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
    let file = match File::open(&artifact_path).await {
        Ok(file) => file,
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
    };

    let size = match file.metadata().await {
        Ok(metadata) => metadata.len(),
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
    };

    HttpResponse::Ok()
        .content_type("application/gzip")
        .insert_header((
            header::CONTENT_DISPOSITION,
            format!(
                "attachment; filename=\"{}.tar.gz\"",
                escape_content_disposition_filename(&artifact.name)
            ),
        ))
        .body(SizedStream::new(size, artifact_stream(file)))
}

/// Reads the artifact file in chunks so that the whole content is never held in
/// memory at once.
fn artifact_stream(file: File) -> impl Stream<Item = Result<Bytes, actix_web::Error>> {
    unfold(file, |mut file| async move {
        let mut buffer = vec![0u8; DOWNLOAD_CHUNK_SIZE];
        match file.read(&mut buffer).await {
            Ok(0) => None,
            Ok(n) => {
                buffer.truncate(n);
                Some((Ok(Bytes::from(buffer)), file))
            }
            Err(e) => Some((Err(ErrorInternalServerError(e)), file)),
        }
    })
}

fn escape_content_disposition_filename(name: &str) -> String {
    name.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .chars()
        .filter(|c| !c.is_control())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{DOWNLOAD_CHUNK_SIZE, artifact_stream, escape_content_disposition_filename};
    use actix_web::web::Bytes;
    use futures::StreamExt;
    use uuid::Uuid;

    #[tokio::test]
    async fn artifact_stream_yields_the_full_file_in_chunks() {
        let path = std::env::temp_dir().join(format!("bld-artifact-stream-{}", Uuid::new_v4()));
        let content: Vec<u8> = (0..DOWNLOAD_CHUNK_SIZE * 2 + 512)
            .map(|i| (i % 251) as u8)
            .collect();
        tokio::fs::write(&path, &content).await.unwrap();

        let file = tokio::fs::File::open(&path).await.unwrap();
        let chunks: Vec<Bytes> = artifact_stream(file)
            .map(|chunk| chunk.unwrap())
            .collect()
            .await;

        let _ = tokio::fs::remove_file(&path).await;

        assert!(chunks.len() > 1);
        assert!(chunks.iter().all(|x| x.len() <= DOWNLOAD_CHUNK_SIZE));
        assert_eq!(chunks.concat(), content);
    }

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

    #[test]
    fn escape_content_disposition_filename_strips_control_characters() {
        assert_eq!(escape_content_disposition_filename("foo\r\nbar"), "foobar");
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
