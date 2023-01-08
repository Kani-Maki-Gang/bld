use crate::context::ContextSender;
use crate::database::pipeline_run_containers::PipelineRunContainers;
use crate::logger::LoggerSender;
use anyhow::{bail, Result};
use bld_config::BldConfig;
use futures::TryStreamExt;
use futures_util::StreamExt;
use shiplift::tty::TtyChunk;
use shiplift::{
    ContainerOptions, Docker, Exec, ExecContainerOptions, ImageListOptions, PullOptions,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tar::Archive;
use tracing::error;

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
        match &self.client {
            Some(client) => Ok(client),
            None => bail!("container not started"),
        }
    }

    fn get_id(&self) -> Result<&str> {
        match &self.id {
            Some(id) => Ok(id),
            None => bail!("container id not found"),
        }
    }

    fn docker(config: &Arc<BldConfig>) -> Result<Docker> {
        let url = config.local.docker_url.parse()?;
        let host = Docker::host(url);
        Ok(host)
    }

    async fn pull(client: &Docker, image: &str, logger: &mut Arc<LoggerSender>) -> Result<()> {
        let options = ImageListOptions::builder().filter_name(image).build();
        let images = client.images().list(&options).await?;

        if images.is_empty() {
            logger.info(format!("Download image: {image}")).await?;

            let options = PullOptions::builder().image(image).build();
            let mut pull_iter = client.images().pull(&options);

            while let Some(Ok(progress)) = pull_iter.next().await {
                logger.write_line(progress.to_string()).await?;
            }
        }

        Ok(())
    }

    async fn create(
        client: &Docker,
        image: &str,
        env: &[String],
        logger: &mut Arc<LoggerSender>,
    ) -> Result<String> {
        Container::pull(client, image, logger).await?;
        let options = ContainerOptions::builder(image).env(env).tty(true).build();
        let info = client.containers().create(&options).await?;
        client.containers().get(&info.id).start().await?;
        Ok(info.id)
    }

    pub async fn new(
        image: &str,
        config: Arc<BldConfig>,
        env: Arc<HashMap<String, String>>,
        logger: Arc<LoggerSender>,
        context: Arc<ContextSender>,
    ) -> Result<Self> {
        let client = Container::docker(&config)?;
        let env: Vec<String> = env.iter().map(|(k, v)| format!("{k}={v}")).collect();
        let id = Container::create(&client, image, &env, &mut logger.clone()).await?;
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
                let _ = self.context
                    .set_container_as_faulted(entity.id.to_owned())
                    .await
                    .map_err(|e| error!("could not set container as faulted, {e}"));
            }
            bail!(e);
        }

        if let Err(e) = client.containers().get(id).delete().await {
            error!("could not stop container, {e}");
            if let Some(entity) = &self.entity {
                let _ = self.context
                    .set_container_as_faulted(entity.id.to_owned())
                    .await
                    .map_err(|e| error!("could not set container as faulted, {e}"));
            }
            bail!(e);
        }

        if let Some(entity) = &self.entity {
            let _ = self.context
                .set_container_as_removed(entity.id.to_owned())
                .await
                .map_err(|e| error!("could not set container as faulted, {e}"));
        }

        Ok(())
    }
}
