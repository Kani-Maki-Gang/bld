use std::sync::Arc;

use anyhow::{bail, Result};
use bollard::{
    image::{BuildImageOptions, CreateImageOptions},
    service::{BuildInfo, CreateImageInfo},
    Docker,
};
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

    async fn pull_image(&self, client: &Docker, logger: &LoggerSender) -> Result<()> {
        let Self::Pull(image) = self else {
            bail!("pulling image isn't allowed with current pipeline configuration");
        };

        let opts = CreateImageOptions {
            from_image: image.as_str(),
            ..Default::default()
        };

        let mut stream = client.create_image(Some(opts), None, None);

        loop {
            let item = stream.try_next().await?;

            match item {
                Some(CreateImageInfo {
                    error: Some(error), ..
                }) => {
                    bail!(error);
                }

                Some(CreateImageInfo {
                    id,
                    status,
                    progress,
                    ..
                }) => {
                    let id = id.unwrap_or_default();
                    let status = status.unwrap_or_default();
                    let progress = progress.unwrap_or_default();
                    let msg = format!("{status} {id} {progress}");
                    logger.write_line(msg).await?;
                }

                None => break,
            }
        }

        Ok(())
    }

    async fn build_image(&self, client: &Docker, logger: &LoggerSender) -> Result<()> {
        let Self::Build {
            name,
            dockerfile,
            tag,
        } = self
        else {
            bail!("building image isn't allowed with current pipeline configuration");
        };

        let image = format!("{name}:{tag}");
        let opts = BuildImageOptions {
            dockerfile: dockerfile.as_str(),
            t: image.as_str(),
            ..Default::default()
        };

        let mut stream = client.build_image(opts, None, None);

        loop {
            let item = stream.try_next().await?;

            match item {
                Some(BuildInfo {
                    error: Some(error), ..
                }) => {
                    bail!(error);
                }

                Some(BuildInfo {
                    id,
                    status,
                    progress,
                    ..
                }) => {
                    let id = id.unwrap_or_default();
                    let status = status.unwrap_or_default();
                    let progress = progress.unwrap_or_default();
                    let msg = format!("{status} {id} {progress}");
                    logger.write_line(msg).await?;
                }

                None => break,
            }
        }

        Ok(())
    }

    pub async fn create(&self, client: &Docker, logger: Arc<LoggerSender>) -> Result<()> {
        match &self {
            Self::Use(_) => Ok(()),
            Self::Pull(_) => self.pull_image(client, logger.as_ref()).await,
            Self::Build { .. } => self.build_image(client, logger.as_ref()).await,
        }
    }
}
