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
    async fn pull(client: &Docker, image: &str, logger: Arc<LoggerSender>) -> Result<String> {
        logger
            .write_line(format!("{:<10}: {image}", "Pull"))
            .await?;

        let pull_opts = PullOptionsBuilder::default().image(image).build();

        let mut stream = client.images().pull(&pull_opts);

        loop {
            match stream.try_next().await {
                Ok(Some(output)) => {
                    let Ok(data) = serde_json::from_value::<StatusData>(output) else {continue};
                    logger.write_line(data.to_string()).await?
                }
                Ok(None) => break,
                Err(_) => continue,
            }
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

        loop {
            match stream.try_next().await {
                Ok(Some(output)) => {
                    let Ok(data) = serde_json::from_value::<StreamData>(output) else {continue};
                    logger.write(data.stream).await?
                }
                Ok(None) => break,
                _ => continue,
            }
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
