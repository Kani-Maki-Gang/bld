use std::{path::Path, sync::Arc};

use anyhow::{anyhow, Result};
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use shiplift::{
    builder::{BuildOptionsBuilder, PullOptionsBuilder},
    Docker,
};

use crate::logger::LoggerSender;

#[derive(Serialize, Deserialize)]
pub struct BuildData {
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
    async fn pull(client: &Docker, image: &str, logger: Arc<LoggerSender>) -> Result<String> {
        logger
            .write_line(format!("{:<10}: {image}", "Pull"))
            .await?;

        let pull_opts = PullOptionsBuilder::default().image(image).build();

        let mut stream = client.images().pull(&pull_opts);

        while let Some(output) = stream.try_next().await? {
            logger.write_line(output.to_string()).await?
        }

        Ok(image.to_owned())
    }

    async fn build(
        client: &Docker,
        name: &str,
        dockerfile: &str,
        tag: &str,
        logger: Arc<LoggerSender>,
    ) -> Result<String> {
        logger
            .write_line(format!("{:<10}: {dockerfile} to {tag}", "Build"))
            .await?;

        let image = format!("{name}:{tag}");

        let path = Path::new(dockerfile);

        let filename = path
            .file_name()
            .and_then(|x| x.to_str())
            .ok_or_else(|| anyhow!("couldn't deduce the file for the dockerfile"))?
            .to_string();

        let parent_dir: String = path
            .parent()
            .and_then(|x| x.to_str())
            .ok_or_else(|| anyhow!("couldn't deduce parent directory of dockerfile"))?
            .to_string();

        let mut build_opts = BuildOptionsBuilder::default()
            .dockerfile(filename)
            .tag(image.to_owned())
            .build();

        build_opts.path = parent_dir;

        let mut stream = client.images().build(&build_opts);

        while let Some(output) = stream.try_next().await? {
            logger.write_line(output.to_string()).await?;
        }

        logger.write_line(String::new()).await?;

        Ok(image)
    }

    pub async fn create(self, client: &Docker, logger: Arc<LoggerSender>) -> Result<String> {
        match self {
            Self::Use(image) => Ok(image),
            Self::Pull(image) => Self::pull(client, &image, logger).await,
            Self::Build {
                name,
                dockerfile,
                tag,
            } => Self::build(client, &name, &dockerfile, &tag, logger).await,
        }
    }
}
