use crate::context::ContextSender;
use crate::database::pipeline_run_containers::PipelineRunContainers;
use crate::logger::LoggerSender;
use anyhow::{anyhow, bail, Result};
use bld_config::BldConfig;
use futures::TryStreamExt;
use futures_util::StreamExt;
use shiplift::builder::{BuildOptionsBuilder, PullOptionsBuilder};
use shiplift::tty::TtyChunk;
use shiplift::{ContainerOptions, Docker, Exec, ExecContainerOptions};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tar::Archive;
use tracing::error;

pub enum Image {
    Use(String),
    Pull(String),
    Build { dockerfile: String, tag: String },
}

impl Image {
    async fn pull(client: &Docker, image: &str, logger: Arc<LoggerSender>) -> Result<String> {
        logger
            .write_line(format!("{:<10}: {image}", "Pull"))
            .await?;

        let pull_opts = PullOptionsBuilder::default().image(image).build();

        let mut stream = client.images().pull(&pull_opts);

        while let Some(result) = stream.next().await {
            if let Err(error) = result {
                logger.write_line(error.to_string()).await?;
                bail!("Unable to pull image");
            }
        }

        Ok(image.to_owned())
    }

    async fn build(
        client: &Docker,
        dockerfile: &str,
        tag: &str,
        logger: Arc<LoggerSender>,
    ) -> Result<String> {
        logger
            .write_line(format!("{:<10}: {dockerfile} to {tag}", "Build"))
            .await?;

        let build_opts = BuildOptionsBuilder::default()
            .dockerfile(dockerfile)
            .tag(tag)
            .build();

        let mut stream = client.images().build(&build_opts);

        while let Some(result) = stream.next().await {
            if let Err(e) = result {
                logger.write_line(e.to_string()).await?;
                bail!("Unable to build image");
            }
        }

        Ok(tag.to_string())
    }

    pub async fn create(self, client: &Docker, logger: Arc<LoggerSender>) -> Result<String> {
        match self {
            Self::Use(image) => Ok(image),
            Self::Pull(image) => Self::pull(client, &image, logger).await,
            Self::Build { dockerfile, tag } => Self::build(client, &dockerfile, &tag, logger).await,
        }
    }
}

pub struct Container {
    pub id: Option<String>,
    pub config: Option<Arc<BldConfig>>,
    pub image: String,
    pub client: Option<Docker>,
    pub logger: Arc<LoggerSender>,
    pub context: Arc<ContextSender>,
    pub entity: Option<PipelineRunContainers>,
}

impl Container {
    fn get_client(&self) -> Result<&Docker> {
        self.client
            .as_ref()
            .ok_or_else(|| anyhow!("container not started"))
    }

    fn get_id(&self) -> Result<&str> {
        self.id
            .as_deref()
            .ok_or_else(|| anyhow!("container id not found"))
    }

    fn docker(config: &Arc<BldConfig>) -> Result<Docker> {
        let url = config.local.docker_url.parse()?;
        let host = Docker::host(url);
        Ok(host)
    }

    async fn create(client: &Docker, image: &str, env: &[String]) -> Result<String> {
        let options = ContainerOptions::builder(image).env(env).tty(true).build();
        let info = client.containers().create(&options).await?;
        client.containers().get(&info.id).start().await?;
        Ok(info.id)
    }

    pub async fn new(
        image: Image,
        config: Arc<BldConfig>,
        env: Arc<HashMap<String, String>>,
        logger: Arc<LoggerSender>,
        context: Arc<ContextSender>,
    ) -> Result<Self> {
        let client = Container::docker(&config)?;
        let env: Vec<String> = env.iter().map(|(k, v)| format!("{k}={v}")).collect();
        let image = image.create(&client, logger.clone()).await?;
        let id = Container::create(&client, &image, &env).await?;
        let entity = context.add_container(id.clone()).await?;
        Ok(Self {
            config: Some(config),
            image: image.to_string(),
            client: Some(client),
            id: Some(id),
            logger,
            context,
            entity,
        })
    }

    pub async fn copy_from(&self, from: &str, to: &str) -> Result<()> {
        let client = self.get_client()?;
        let container = client.containers().get(self.get_id()?);
        let bytes = container.copy_from(Path::new(from)).try_concat().await?;
        let mut archive = Archive::new(&bytes[..]);
        archive.unpack(Path::new(to))?;
        Ok(())
    }

    pub async fn copy_into(&self, from: &str, to: &str) -> Result<()> {
        let client = self.get_client()?;
        let container = client.containers().get(self.get_id()?);
        let content = std::fs::read(from)?;
        container.copy_file_into(to, &content).await?;
        Ok(())
    }

    pub async fn sh(&self, working_dir: &Option<String>, input: &str) -> Result<()> {
        let client = self.get_client()?;
        let id = self.get_id()?;
        let input = working_dir
            .as_ref()
            .map(|wd| format!("cd {wd} && {input}"))
            .or_else(|| Some(input.to_string()))
            .unwrap();

        let options = ExecContainerOptions::builder()
            .cmd(vec!["bash", "-c", &input])
            .attach_stdout(true)
            .attach_stderr(true)
            .build();

        let exec = Exec::create(client, id, &options).await?;
        let mut exec_stream = exec.start();

        while let Some(result) = exec_stream.next().await {
            let chunk = match result {
                Ok(TtyChunk::StdOut(bytes)) => String::from_utf8(bytes)?,
                Ok(TtyChunk::StdErr(bytes)) => String::from_utf8(bytes)?,
                Ok(TtyChunk::StdIn(_)) => unreachable!(),
                Err(e) => bail!(e),
            };

            self.logger.write(chunk).await?;
        }

        let inspect = exec.inspect().await?;
        match inspect.exit_code {
            Some(code) if code > 0 => bail!("command finished with exit code: {code}"),
            _ => {}
        }

        Ok(())
    }

    pub async fn keep_alive(&self) -> Result<()> {
        let id = self.get_id()?;
        self.context.keep_alive(id.to_string()).await
    }

    pub async fn dispose(&self) -> Result<()> {
        let client = self.get_client()?;
        let id = self.get_id()?;

        if let Err(e) = client.containers().get(id).stop(None).await {
            error!("could not stop container, {e}");
            if let Some(entity) = &self.entity {
                let _ = self
                    .context
                    .set_container_as_faulted(entity.id.to_owned())
                    .await
                    .map_err(|e| error!("could not set container as faulted, {e}"));
            }
            bail!(e);
        }

        if let Err(e) = client.containers().get(id).delete().await {
            error!("could not stop container, {e}");
            if let Some(entity) = &self.entity {
                let _ = self
                    .context
                    .set_container_as_faulted(entity.id.to_owned())
                    .await
                    .map_err(|e| error!("could not set container as faulted, {e}"));
            }
            bail!(e);
        }

        if let Some(entity) = &self.entity {
            let _ = self
                .context
                .set_container_as_removed(entity.id.to_owned())
                .await
                .map_err(|e| error!("could not set container as faulted, {e}"));
        }

        Ok(())
    }
}
