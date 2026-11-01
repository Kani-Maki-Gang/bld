use crate::extractors::User;
use actix_web::web::{Data, Query};
use actix_web::{HttpResponse, Responder, get};
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::fs::FileSystem;
use bld_models::dtos::PipelineQueryParams;
use bld_pkg::PackageManager;
use bld_runner::{VersionedFile, VersionedFileLoader};
use std::sync::Arc;
use tracing::info;

#[get("/v1/check")]
pub async fn get(
    _user: User,
    config: Data<BldConfig>,
    fs: Data<FileSystem>,
    package_manager: Data<PackageManager>,
    params: Query<PipelineQueryParams>,
) -> impl Responder {
    info!("Reached handler for /check route");
    let result = do_check(
        config.into_inner(),
        fs.into_inner(),
        package_manager.into_inner(),
        &params,
    )
    .await;
    match result {
        Ok(_) => HttpResponse::Ok().json(""),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

async fn do_check(
    config: Arc<BldConfig>,
    fs: Arc<FileSystem>,
    package_manager: Arc<PackageManager>,
    params: &PipelineQueryParams,
) -> Result<()> {
    let loader = VersionedFileLoader::new(&package_manager, &fs, true);
    let metadata = loader.load(&params.pipeline).await?;
    metadata
        .file
        .validate_with_verbose_errors(config.clone(), fs.clone(), package_manager.clone())
        .await?;
    VersionedFile::check_dependency_cycles(&config, &fs, &package_manager, &params.pipeline).await
}

#[cfg(test)]
mod tests {
    use super::do_check;
    use bld_config::{BldConfig, BldPackages};
    use bld_core::fs::FileSystem;
    use bld_models::dtos::PipelineQueryParams;
    use bld_pkg::PackageManager;
    use std::{path::PathBuf, sync::Arc};

    struct TestEnv {
        dir: PathBuf,
        config: Arc<BldConfig>,
    }

    impl TestEnv {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("bld_server_check_{name}"));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            let mut config = BldConfig {
                root_dir: dir.to_string_lossy().into_owned(),
                ..Default::default()
            };
            config.local.packages = BldPackages {
                cache: dir.join("packages").to_string_lossy().into_owned(),
                strict_sync: false,
            };
            Self {
                dir,
                config: Arc::new(config),
            }
        }

        async fn check(&self, raw: &str) -> Result<(), String> {
            std::fs::write(self.dir.join("pipeline.yaml"), raw).unwrap();
            do_check(
                self.config.clone(),
                Arc::new(FileSystem::local(self.config.clone())),
                Arc::new(PackageManager::new(self.config.clone())),
                &PipelineQueryParams::new("pipeline.yaml"),
            )
            .await
            .map_err(|e| e.to_string())
        }
    }

    impl Drop for TestEnv {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    #[tokio::test]
    async fn check_accepts_a_valid_pipeline() {
        let env = TestEnv::new("valid");
        let raw = "version: 3\ntype: pipeline\njobs:\n  main:\n    runs_on: machine\n    steps:\n      - id: s\n        run: echo hello\n";
        assert_eq!(env.check(raw).await, Ok(()));
    }

    #[tokio::test]
    async fn check_reports_expression_errors() {
        let env = TestEnv::new("expression_error");
        let raw = "version: 3\ntype: pipeline\njobs:\n  main:\n    runs_on: machine\n    steps:\n      - id: s\n        run: echo ${{ inputs.missing }}\n";
        let error = env.check(raw).await.unwrap_err();
        assert!(error.starts_with("Expression errors"), "{error}");
    }
}
