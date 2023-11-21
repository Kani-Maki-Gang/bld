use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{bail, Result};
use bld_config::{path, BldConfig};
use bld_docker::{
    apis::{
        configuration::Configuration,
        container_api::{
            container_archive, container_create, container_delete, container_start,
            put_container_archive,
        }, exec_api::container_exec,
    },
    models::{ContainerCreateRequest, ExecConfig},
};
use reqwest::Client;
use tar::Archive;
use tracing::error;

use crate::{
    context::ContextSender, database::pipeline_run_containers::PipelineRunContainers,
    logger::LoggerSender,
};

use super::Image;

pub struct Container {
    pub id: String,
    pub docker: Configuration,
    pub image: String,
    pub config: Arc<BldConfig>,
    pub context: Arc<ContextSender>,
    pub entity: Option<PipelineRunContainers>,
    pub environment: Vec<String>,
}

impl Container {
    fn docker(config: &Arc<BldConfig>) -> Result<Configuration> {
        let base_path = config.local.docker_url.to_owned();
        let client = Client::new();
        let configuration = Configuration {
            base_path,
            client,
            ..Default::default()
        };
        Ok(configuration)
    }

    async fn create(docker: &Configuration, image: String, env: Vec<String>) -> Result<String> {
        let mut request = ContainerCreateRequest::new();
        request.image = Some(image);
        request.env = Some(env);
        request.tty = Some(true);

        let response = container_create(docker, request, None, None).await?;

        container_start(docker, &response.id, None).await?;

        Ok(response.id)
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

    pub async fn new(
        image: Image,
        config: Arc<BldConfig>,
        pipeline_env: &HashMap<String, String>,
        env: Arc<HashMap<String, String>>,
        logger: Arc<LoggerSender>,
        context: Arc<ContextSender>,
    ) -> Result<Self> {
        let docker = Container::docker(&config)?;
        let environment = Self::create_environment(pipeline_env, env);
        let image = image.create(&docker, logger.clone()).await?;
        let id = Container::create(&docker, image.clone(), environment.clone()).await?;
        let entity = context.add_container(id.clone()).await?;
        Ok(Self {
            id,
            docker,
            config,
            image,
            context,
            entity,
            environment,
        })
    }

    pub async fn copy_from(&self, from: &str, to: &str) -> Result<()> {
        let data = container_archive(&self.docker, &self.id, from).await?;
        let mut archive = Archive::new(&data[..]);
        archive.unpack(Path::new(to))?;
        Ok(())
    }

    pub async fn copy_into(&self, from: &str, to: &str) -> Result<()> {
        let from_path = path![from];
        put_container_archive(&self.docker, &self.id, to, from_path, None, None).await?;
        Ok(())
    }

    pub async fn sh(
        &self,
        _logger: Arc<LoggerSender>,
        working_dir: &Option<String>,
        input: &str,
    ) -> Result<()> {
        let input = working_dir
            .as_ref()
            .map(|wd| format!("cd {wd} && {input}"))
            .or_else(|| Some(input.to_string()))
            .unwrap();

        let env = self.environment.iter().map(String::clone).collect();
        let mut exec_config = ExecConfig::new();
        exec_config.cmd = Some(vec!["bash".to_string(), "-c".to_string(), input]);
        exec_config.env = Some(env);
        exec_config.attach_stdout = Some(true);
        exec_config.attach_stderr = Some(true);
        let _ = container_exec(&self.docker, &self.id, exec_config).await?;

        // let options = ExecContainerOptions::builder()
        //     .cmd(vec!["bash", "-c", &input])
        //     .env(env)
        //     .attach_stdout(true)
        //     .attach_stderr(true)
        //     .build();

        // let exec = Exec::create(client, id, &options).await?;
        // let mut exec_stream = exec.start();

        // while let Some(result) = exec_stream.next().await {
        //     let chunk = match result {
        //         Ok(TtyChunk::StdOut(bytes)) => String::from_utf8(bytes)?,
        //         Ok(TtyChunk::StdErr(bytes)) => String::from_utf8(bytes)?,
        //         Ok(TtyChunk::StdIn(_)) => unreachable!(),
        //         Err(e) => bail!(e),
        //     };

        //     logger.write(chunk).await?;
        // }

        // let inspect = exec.inspect().await?;
        // match inspect.exit_code {
        //     Some(code) if code > 0 => bail!("command finished with exit code: {code}"),
        //     _ => {}
        // }

        Ok(())
    }

    pub async fn keep_alive(&self) -> Result<()> {
        self.context.keep_alive(self.id.to_owned()).await
    }

    pub async fn dispose(&self) -> Result<()> {
        if let Err(e) = container_delete(&self.docker, &self.id, None, Some(true), None).await {
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
