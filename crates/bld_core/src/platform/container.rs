use std::{collections::HashMap, path::Path, path::PathBuf, sync::Arc};

use anyhow::{anyhow, bail, Result};
use bld_config::{path, BldConfig};
use bld_entities::pipeline_run_containers::PipelineRunContainers;
use bollard::{
    container::{
        Config as ContainerConfig, CreateContainerOptions, DownloadFromContainerOptions, LogOutput,
        StartContainerOptions, UploadToContainerOptions,
    },
    exec::{CreateExecOptions, StartExecResults},
    Docker,
};
use futures::StreamExt;
use tar::{Archive, Builder};
use tracing::{debug, error};
use uuid::Uuid;

use crate::{context::ContextSender, logger::LoggerSender};

use super::{docker, Image};

pub struct ContainerOptions<'a> {
    pub config: Arc<BldConfig>,
    pub docker_url: Option<&'a str>,
    pub image: Image<'a>,
    pub pipeline_env: &'a HashMap<String, String>,
    pub env: Arc<HashMap<String, String>>,
    pub logger: Arc<LoggerSender>,
    pub context: Arc<ContextSender>,
}

pub struct Container {
    pub id: String,
    pub name: String,
    pub config: Option<Arc<BldConfig>>,
    pub client: Docker,
    pub context: Arc<ContextSender>,
    pub entity: Option<PipelineRunContainers>,
    pub environment: Vec<String>,
}

impl Container {
    async fn create(client: &Docker, image: &str, env: Vec<&str>) -> Result<(String, String)> {
        let name = Uuid::new_v4().to_string();
        let options = CreateContainerOptions {
            name: &name,
            platform: None,
        };
        let config = ContainerConfig {
            image: Some(image),
            tty: Some(true),
            env: Some(env),
            ..Default::default()
        };

        let create_resp = client.create_container(Some(options), config).await?;
        client
            .start_container(&name, None::<StartContainerOptions<String>>)
            .await?;

        Ok((create_resp.id, name))
    }

    fn create_environment(
        pipeline_env: &HashMap<String, String>,
        env: Arc<HashMap<String, String>>,
    ) -> Vec<String> {
        let mut map = HashMap::new();

        for (k, v) in pipeline_env.iter() {
            map.insert(k.to_owned(), v.to_owned());
        }

        for (k, v) in env.iter() {
            map.insert(k.to_owned(), v.to_owned());
        }

        map.iter().map(|(k, v)| format!("{k}={v}")).collect()
    }

    pub async fn new(options: ContainerOptions<'_>) -> Result<Self> {
        let client = docker(options.config.as_ref(), options.docker_url)?;
        debug!("creating container environement");
        let env = Self::create_environment(options.pipeline_env, options.env);
        let container_env = env.iter().map(AsRef::as_ref).collect();
        options
            .image
            .create(&client, options.logger.clone())
            .await?;
        let (id, name) = Container::create(&client, options.image.name(), container_env).await?;
        let entity = options.context.add_container(id.clone()).await?;
        Ok(Self {
            id,
            name,
            config: Some(options.config),
            client,
            context: options.context,
            entity,
            environment: env,
        })
    }

    pub async fn copy_from(&self, from: &str, to: &str) -> Result<()> {
        let options = DownloadFromContainerOptions { path: from };
        let mut stream = self
            .client
            .download_from_container(&self.name, Some(options));

        let mut bytes = vec![];
        while let Some(item) = stream.next().await {
            let mut item: Vec<u8> = item?.into();
            bytes.append(&mut item);
        }

        let mut archive = Archive::new(&bytes[..]);
        archive.unpack(Path::new(to))?;
        Ok(())
    }

    pub async fn copy_into(&self, from: &str, to: &str) -> Result<()> {
        let mut tar = Builder::new(Vec::new());
        let path = path![from];

        let filename = path
            .file_name()
            .ok_or_else(|| anyhow!("unable to retrieve filename for path {from}"))?;

        if path.is_file() {
            tar.append_path_with_name(from, filename)?;
        } else {
            tar.append_dir_all(filename, from)?;
        }
        let content = tar.into_inner()?;

        let options = UploadToContainerOptions {
            path: to,
            ..Default::default()
        };
        self.client
            .upload_to_container(&self.name, Some(options), content.into())
            .await?;

        Ok(())
    }

    pub async fn sh(
        &self,
        logger: Arc<LoggerSender>,
        working_dir: &Option<String>,
        input: &str,
    ) -> Result<()> {
        let input = working_dir
            .as_ref()
            .map(|wd| format!("cd {wd} && {input}"))
            .or_else(|| Some(input.to_string()))
            .unwrap();

        let env = self.environment.iter().map(String::as_str).collect();
        let options = CreateExecOptions {
            cmd: Some(vec!["bash", "-c", &input]),
            env: Some(env),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            ..Default::default()
        };

        let exec = self.client.create_exec(&self.name, options).await?;
        let exec_stream = self.client.start_exec(&exec.id, None).await?;

        let StartExecResults::Attached { mut output, .. } = exec_stream else {
            return Ok(());
        };

        while let Some(result) = output.next().await {
            let Ok(output) = result else {
                continue;
            };

            let chunk: Vec<u8> = match output {
                LogOutput::StdOut { message } => message.into(),
                LogOutput::StdErr { message } => message.into(),
                LogOutput::StdIn { .. } | LogOutput::Console { .. } => continue,
            };

            let chunk_str = String::from_utf8(chunk)?;

            logger.write(chunk_str).await?;
        }

        let inspect = self.client.inspect_exec(&exec.id).await?;
        let Some(exit_code) = inspect.exit_code else {
            bail!("unable to confirm exit code");
        };

        if exit_code != 0 {
            bail!("command finished with exit code: {exit_code}");
        }

        Ok(())
    }

    pub async fn keep_alive(&self) -> Result<()> {
        self.context.keep_alive(self.id.clone()).await
    }

    pub async fn dispose(&self) -> Result<()> {
        if let Err(e) = self.client.stop_container(&self.name, None).await {
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

        if let Err(e) = self.client.remove_container(&self.name, None).await {
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
