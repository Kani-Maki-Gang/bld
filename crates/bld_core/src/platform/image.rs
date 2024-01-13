use std::{io::Write, sync::Arc};

use anyhow::{bail, Result};
use bollard::{
    image::{BuildImageOptions, CreateImageOptions},
    service::{BuildInfo, CreateImageInfo},
    Docker,
};
use flate2::{write::GzEncoder, Compression};
use futures::TryStreamExt;
use tar::{Builder, Header};
use tokio::fs::read_to_string;

use crate::logger::LoggerSender;

pub struct PullImage<'a>(&'a str);

impl<'a> PullImage<'a> {
    pub async fn pull(&self, client: &Docker, logger: &LoggerSender) -> Result<()> {
        let image = self.0;
        let opts = CreateImageOptions {
            from_image: image,
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
}

pub struct BuildImage<'a> {
    name: String,
    dockerfile: &'a str,
}

impl<'a> BuildImage<'a> {
    pub fn new(name: &str, dockerfile: &'a str, tag: &str) -> Self {
        let name = format!("{name}:{tag}");
        Self { name, dockerfile }
    }

    pub async fn build(&self, client: &Docker, logger: &LoggerSender) -> Result<()> {
        let content = read_to_string(&self.dockerfile).await?;

        let mut header = Header::new_gnu();
        header.set_path("Dockerfile")?;
        header.set_size(content.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();

        let mut tar = Builder::new(vec![]);
        tar.append(&header, content.as_bytes())?;

        let uncompressed = tar.into_inner()?;
        let mut gz = GzEncoder::new(vec![], Compression::default());
        gz.write_all(&uncompressed)?;
        let compressed = gz.finish()?;

        let opts = BuildImageOptions {
            t: self.name.as_str(),
            ..Default::default()
        };

        let mut stream = client.build_image(opts, None, Some(compressed.into()));

        loop {
            let item = stream.try_next().await?;

            match item {
                Some(BuildInfo {
                    error: Some(error), ..
                }) => {
                    bail!(error);
                }

                Some(BuildInfo {
                    stream: Some(stream),
                    ..
                }) => {
                    logger.write(stream).await?;
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
}

pub enum Image<'a> {
    Use(&'a str),
    Pull(PullImage<'a>),
    Build(BuildImage<'a>),
}

impl<'a> Image<'a> {
    pub fn pull(image: &'a str) -> Self {
        Self::Pull(PullImage(image))
    }

    pub fn build(name: &str, dockerfile: &'a str, tag: &str) -> Self {
        Self::Build(BuildImage::new(name, dockerfile, tag))
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Use(image) | Self::Pull(PullImage(image)) => image,
            Self::Build(BuildImage { name, .. }) => name.as_str(),
        }
    }

    pub async fn create(&self, client: &Docker, logger: Arc<LoggerSender>) -> Result<()> {
        match &self {
            Self::Use(_) => Ok(()),
            Self::Pull(instance) => instance.pull(client, logger.as_ref()).await,
            Self::Build(instance) => instance.build(client, logger.as_ref()).await,
        }
    }
}
