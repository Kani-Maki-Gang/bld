use std::{collections::HashMap, path::Path, path::PathBuf, sync::Arc};

use anyhow::{Result, anyhow, bail};
use bld_config::{BldConfig, definitions::BLD_OUTPUTS_ENV_VAR_V3, path};
use bld_utils::variables::parse_variables_iter;
use bollard::{
    Docker,
    container::{
        Config as ContainerConfig, CreateContainerOptions, DownloadFromContainerOptions, LogOutput,
        StartContainerOptions, UploadToContainerOptions,
    },
    exec::{CreateExecOptions, StartExecResults},
};
use futures::StreamExt;
use tar::{Archive, Builder};
use tracing::{debug, error};
use uuid::Uuid;

use crate::logger::Logger;

use super::{Image, context::PlatformContext, docker};

pub struct ContainerOptions<'a> {
    pub config: Arc<BldConfig>,
    pub docker_url: Option<&'a str>,
    pub image: Image<'a>,
    pub pipeline_env: &'a HashMap<String, String>,
    pub env: Arc<HashMap<String, String>>,
    pub logger: Arc<Logger>,
    pub context: PlatformContext,
}

pub struct Container {
    pub id: String,
    pub name: String,
    pub config: Option<Arc<BldConfig>>,
    pub client: Docker,
    pub context: PlatformContext,
    pub env: Vec<String>,
    pub outputs_dir: PathBuf,
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

    fn create_env(
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

    pub async fn new(mut options: ContainerOptions<'_>) -> Result<Self> {
        let client = docker(options.config.as_ref(), options.docker_url)?;
        debug!("creating container environement");
        let env = Self::create_env(options.pipeline_env, options.env);
        let container_env = env.iter().map(AsRef::as_ref).collect();
        options
            .image
            .create(&client, options.logger.as_ref())
            .await?;
        let (id, name) = Container::create(&client, options.image.name(), container_env).await?;

        options.context.add(&id).await?;

        let instance = Self {
            id,
            name,
            config: Some(options.config),
            client,
            context: options.context,
            env,
            outputs_dir: path!["tmp", "outputs"],
        };

        instance
            .run_internal_cmd(vec![
                "mkdir",
                "-p",
                &instance.outputs_dir.display().to_string(),
            ])
            .await?;

        Ok(instance)
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

    async fn run_internal_cmd(&self, cmd: Vec<&str>) -> Result<String> {
        debug!("running internal container command");
        let options = CreateExecOptions {
            cmd: Some(cmd),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            ..Default::default()
        };

        let exec = self.client.create_exec(&self.name, options).await?;
        let exec_stream = self.client.start_exec(&exec.id, None).await?;

        let StartExecResults::Attached { mut output, .. } = exec_stream else {
            debug!("unable to attach stdout for internal container command");
            bail!("unable to communicate with container");
        };

        let mut stdout = String::new();
        let mut stderr = String::new();
        while let Some(result) = output.next().await {
            let Ok(output) = result else {
                continue;
            };
            match output {
                LogOutput::StdIn { .. } | LogOutput::Console { .. } => {
                    continue;
                }
                LogOutput::StdOut { message } => {
                    let chunk_str = String::from_utf8(message.into())?;
                    stdout.push_str(&chunk_str);
                }
                LogOutput::StdErr { message } => {
                    let chunk_str = String::from_utf8(message.into())?;
                    stderr.push_str(&chunk_str);
                }
            };
        }

        let inspect = self.client.inspect_exec(&exec.id).await?;
        let Some(exit_code) = inspect.exit_code else {
            bail!("unable to confirm exit code for creating outputs file");
        };

        if exit_code != 0 {
            debug!("failed to run internal container command due to {stderr}");
            bail!("command finished with exit code: {exit_code}");
        }

        debug!("successfully run internal container command");
        Ok(stdout)
    }

    pub async fn sh(
        &self,
        logger: Arc<Logger>,
        working_dir: &Option<String>,
        input: &str,
    ) -> Result<HashMap<String, String>> {
        let outputs_path = path![&self.outputs_dir, Uuid::new_v4().to_string()]
            .display()
            .to_string();

        // self.run_internal_cmd(vec!["touch", &outputs_path]).await?;

        let input = working_dir
            .as_ref()
            .map(|wd| format!("cd {wd} && {input}"))
            .or_else(|| Some(input.to_string()))
            .unwrap();

        let outputs_env = format!("{BLD_OUTPUTS_ENV_VAR_V3}={}", outputs_path);
        let mut env: Vec<&str> = self.env.iter().map(String::as_str).collect();
        env.push(&outputs_env);

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
            return Ok(HashMap::new());
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

        // Ignoring errors here but logging them since many steps don't create outputs and thus
        // the outputs files might not even exist
        let outputs = self
            .run_internal_cmd(vec!["cat", &outputs_path])
            .await
            .unwrap_or_default();
        let outputs = parse_variables_iter(outputs.lines());

        Ok(outputs)
    }

    pub async fn keep_alive(&self) -> Result<()> {
        self.context.keep_alive().await
    }

    pub async fn dispose(&self) -> Result<()> {
        if let Err(e) = self.client.stop_container(&self.name, None).await {
            error!("could not stop container, {e}");
            let _ = self
                .context
                .set_as_faulted()
                .await
                .map_err(|e| error!("could not set container as faulted, {e}"));
            bail!(e);
        }

        if let Err(e) = self.client.remove_container(&self.name, None).await {
            error!("could not stop container, {e}");
            let _ = self
                .context
                .set_as_faulted()
                .await
                .map_err(|e| error!("could not set container as faulted, {e}"));
            bail!(e);
        }

        let _ = self
            .context
            .set_as_removed()
            .await
            .map_err(|e| error!("could not set container as faulted, {e}"));

        Ok(())
    }
}
