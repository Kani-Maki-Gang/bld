use std::sync::Arc;

use anyhow::Result;
use bollard::{image::BuildImageOptions, service::BuildInfo, Docker};
use futures::TryStreamExt;
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
    pub fn name(&self) -> String {
        match self {
            Self::Use(image) | Self::Pull(image) => image.to_owned(),
            Self::Build { name, tag, .. } => format!("{name}:{tag}"),
        }
    }

    pub async fn create(&self, client: &Docker, logger: Arc<LoggerSender>) -> Result<()> {
        let mut build_opts = BuildImageOptions::default();

        let mut stream = match &self {
            Self::Use(_) => return Ok(()),

            Self::Pull(image) => {
                build_opts.t = image.to_owned();
                build_opts.pull = true;
                client.build_image(build_opts, None, None)
            }

            Self::Build {
                name,
                dockerfile,
                tag,
            } => {
                let image = format!("{name}:{tag}");
                build_opts.dockerfile = dockerfile.to_owned();
                build_opts.t = image.to_owned();
                client.build_image(build_opts, None, None)
            }
        };

        loop {
            let Ok(item) = stream.try_next().await else {
                continue;
            };

            match item {
                Some(BuildInfo {
                    progress: Some(value),
                    ..
                }) => {
                    logger.write_line(value).await?;
                }
                Some(_) => continue,
                None => break,
            }
        }

        Ok(())
    }
}
