use anyhow::{bail, Result};
use bld_config::BldConfig;
use bld_core::context::Context;
use bld_core::execution::Execution;
use bld_core::logger::Logger;
use futures::TryStreamExt;
use futures_util::StreamExt;
use shiplift::tty::TtyChunk;
use shiplift::{
    ContainerOptions, Docker, Exec, ExecContainerOptions, ImageListOptions, PullOptions,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tar::Archive;
use tracing::{debug, error};

type AtomicLogger = Arc<Mutex<Logger>>;

pub struct Container {
    pub id: Option<String>,
    pub config: Option<Arc<BldConfig>>,
    pub image: String,
    pub client: Option<Docker>,
    pub logger: AtomicLogger,
    pub containers: Arc<Mutex<Context>>,
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

    async fn pull(client: &Docker, image: &str, logger: &mut AtomicLogger) -> Result<()> {
        let options = ImageListOptions::builder().filter_name(image).build();
        let images = client.images().list(&options).await?;
        if images.is_empty() {
            {
                let mut logger = logger.lock().unwrap();
                logger.info(&format!("Download image: {image}"));
            }
            let options = PullOptions::builder().image(image).build();
            let mut pull_iter = client.images().pull(&options);
            while let Some(Ok(progress)) = pull_iter.next().await {
                let mut logger = logger.lock().unwrap();
                logger.dumpln(&progress.to_string());
            }
        }
        Ok(())
    }

    async fn create(
        client: &Docker,
        image: &str,
        env: &[String],
        logger: &mut AtomicLogger,
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
        logger: AtomicLogger,
        containers: Arc<Mutex<Context>>,
    ) -> Result<Self> {
        let client = Container::docker(&config)?;
        let env: Vec<String> = env.iter().map(|(k, v)| format!("{k}={v}")).collect();
        let id = Container::create(&client, image, &env, &mut logger.clone()).await?;
        {
            let mut containers = containers.lock().unwrap();
            containers.add(&id)?;
        }
        Ok(Self {
            config: Some(config),
            image: image.to_string(),
            client: Some(client),
            id: Some(id),
            logger,
            containers,
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

    pub async fn sh(
        &self,
        working_dir: &Option<String>,
        input: &str,
        ex: Arc<Mutex<Execution>>,
    ) -> Result<()> {
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
            {
                let exec = ex.lock().unwrap();
                exec.check_stop_signal()?
            }

            let chunk = match result {
                Ok(TtyChunk::StdOut(bytes)) => String::from_utf8(bytes)?,
                Ok(TtyChunk::StdErr(bytes)) => String::from_utf8(bytes)?,
                Ok(TtyChunk::StdIn(_)) => unreachable!(),
                Err(e) => bail!(e),
            };

            let mut logger = self.logger.lock().unwrap();
            logger.dump(&chunk);
        }

        let inspect = exec.inspect().await?;
        match inspect.exit_code {
            Some(code) if code > 0 => bail!("command finished with exit code: {code}"),
            _ => {}
        }

        Ok(())
    }

    pub fn keep_alive(&self) -> Result<()> {
        let id = self.get_id()?;
        let mut containers = self.containers.lock().unwrap();
        containers.keep_alive(id)
    }

    pub async fn dispose(&self) -> Result<()> {
        let client = self.get_client()?;
        let id = self.get_id()?;

        if let Err(e) = client.containers().get(id).stop(None).await {
            error!("could not stop container, {e}");
            let mut containers = self.containers.lock().unwrap();
            containers.faulted(id)?;
            bail!(e);
        }

        if let Err(e) = client.containers().get(id).delete().await {
            error!("could not stop container, {e}");
            let mut containers = self.containers.lock().unwrap();
            containers.faulted(id)?;
            bail!(e);
        }

        let mut containers = self.containers.lock().unwrap();
        containers.remove(id)?;

        Ok(())
    }
}
