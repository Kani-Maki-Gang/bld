use std::sync::Arc;

use anyhow::Result;
use bld_docker::apis::{configuration::Configuration, image_api::image_build};
use serde::{Deserialize, Serialize};

use crate::logger::LoggerSender;

#[derive(Serialize, Deserialize)]
pub struct StatusData {
    id: String,
    status: String,
    progress: Option<String>,
}

impl ToString for StatusData {
    fn to_string(&self) -> String {
        match &self.progress {
            Some(progress) => format!("{} {} {progress}", self.id, self.status),
            None => format!("{} {}", self.id, self.status),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct StreamData {
    stream: String,
}

pub enum Image {
    Use(String),
    Pull(String),
    Build {
        name: String,
        dockerfile: String,
        tag: String,
    },
}

impl Image {
    async fn pull(
        docker: &Configuration,
        image: &str,
        logger: Arc<LoggerSender>,
    ) -> Result<String> {
        logger
            .write_line(format!("{:<15}: {image}", "Pull"))
            .await?;

        image_build(
            docker,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(image),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await?;

        Ok(image.to_owned())
    }

    async fn build(
        docker: &Configuration,
        name: &str,
        dockerfile: &str,
        tag: &str,
        logger: Arc<LoggerSender>,
    ) -> Result<String> {
        logger
            .write_line(format!("{:<15}: {dockerfile} to {tag}", "Build"))
            .await?;

        let image = format!("{name}:{tag}");

        image_build(
            docker,
            Some(dockerfile),
            Some(&image),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await?;

        Ok(image)
    }

    pub async fn create(self, docker: &Configuration, logger: Arc<LoggerSender>) -> Result<String> {
        match self {
            Self::Use(image) => Ok(image),
            Self::Pull(image) => Self::pull(docker, &image, logger).await,
            Self::Build {
                name,
                dockerfile,
                tag,
            } => Self::build(docker, &name, &dockerfile, &tag, logger).await,
        }
    }
}
