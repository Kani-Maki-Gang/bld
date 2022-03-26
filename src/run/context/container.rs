use crate::config::BldConfig;
use crate::persist::Logger;
use crate::run::CheckStopSignal;
use anyhow::anyhow;
use futures::TryStreamExt;
use futures_util::StreamExt;
use shiplift::tty::TtyChunk;
use shiplift::{ContainerOptions, Docker, ExecContainerOptions, ImageListOptions, PullOptions};
use std::path::Path;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use tar::Archive;

type AtomicRecv = Arc<Mutex<Receiver<bool>>>;

pub struct Container {
    pub config: Option<Arc<BldConfig>>,
    pub img: String,
    pub client: Option<Docker>,
    pub id: Option<String>,
    pub lg: Arc<Mutex<dyn Logger>>,
}

impl Container {
    fn get_client(&self) -> anyhow::Result<&Docker> {
        match &self.client {
            Some(client) => Ok(client),
            None => Err(anyhow!("container not started")),
        }
    }

    fn get_id(&self) -> anyhow::Result<&str> {
        match &self.id {
            Some(id) => Ok(id),
            None => Err(anyhow!("container id not found")),
        }
    }

    fn docker(config: &Arc<BldConfig>) -> anyhow::Result<Docker> {
        let url = config.local.docker_url.parse()?;
        let host = Docker::host(url);
        Ok(host)
    }

    async fn pull(
        client: &Docker,
        image: &str,
        logger: &mut Arc<Mutex<dyn Logger>>,
    ) -> anyhow::Result<()> {
        let options = ImageListOptions::builder().filter_name(image).build();
        let images = client.images().list(&options).await?;
        if images.is_empty() {
            {
                let mut logger = logger.lock().unwrap();
                logger.info(&format!("Download image: {}", image));
            }
            let options = PullOptions::builder().image(image).build();
            let mut pull_iter = client.images().pull(&options);
            while let Some(progress) = pull_iter.next().await {
                let info = progress?;
                {
                    let mut logger = logger.lock().unwrap();
                    logger.dumpln(&info.to_string());
                }
                sleep(Duration::from_millis(100));
            }
        }
        Ok(())
    }

    async fn create(
        client: &Docker,
        image: &str,
        logger: &mut Arc<Mutex<dyn Logger>>,
    ) -> anyhow::Result<String> {
        Container::pull(client, image, logger).await?;
        let options = ContainerOptions::builder(image).tty(true).build();
        let info = client.containers().create(&options).await?;
        client.containers().get(&info.id).start().await?;
        Ok(info.id)
    }

    pub async fn new(
        img: &str,
        cfg: Arc<BldConfig>,
        lg: Arc<Mutex<dyn Logger>>,
    ) -> anyhow::Result<Self> {
        let client = Container::docker(&cfg)?;
        let id = Container::create(&client, img, &mut lg.clone()).await?;
        Ok(Self {
            config: Some(cfg),
            img: img.to_string(),
            client: Some(client),
            id: Some(id),
            lg,
        })
    }

    pub async fn copy_from(&self, from: &str, to: &str) -> anyhow::Result<()> {
        let client = self.get_client()?;
        let container = client.containers().get(self.get_id()?);
        let bytes = container.copy_from(Path::new(from)).try_concat().await?;
        let mut archive = Archive::new(&bytes[..]);
        archive.unpack(Path::new(to))?;
        Ok(())
    }

    pub async fn copy_into(&self, from: &str, to: &str) -> anyhow::Result<()> {
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
        cm: &Option<AtomicRecv>,
    ) -> anyhow::Result<()> {
        let client = self.get_client()?;
        let id = self.get_id()?;
        let input = working_dir
            .as_ref()
            .map(|wd| format!("cd {} && {}", &wd, input))
            .or_else(|| Some(input.to_string()))
            .unwrap();
        let cmd = vec!["bash", "-c", &input];
        let options = ExecContainerOptions::builder()
            .cmd(cmd)
            .attach_stdout(true)
            .attach_stderr(true)
            .build();
        let container = client.containers().get(id);
        let mut exec_iter = container.exec(&options);
        while let Some(result) = exec_iter.next().await {
            cm.check_stop_signal()?;
            let chunk = match result {
                Ok(TtyChunk::StdOut(bytes)) => String::from_utf8(bytes).unwrap(),
                Ok(TtyChunk::StdErr(bytes)) => String::from_utf8(bytes).unwrap(),
                Ok(TtyChunk::StdIn(_)) => unreachable!(),
                Err(e) => return Err(anyhow!(e)),
            };
            {
                let mut logger = self.lg.lock().unwrap();
                logger.dump(&chunk);
            }
            sleep(Duration::from_millis(100));
        }
        Ok(())
    }

    pub async fn dispose(&self) -> anyhow::Result<()> {
        let client = self.get_client()?;
        let id = self.get_id()?;
        client.containers().get(id).stop(None).await?;
        client.containers().get(id).delete().await?;
        Ok(())
    }
}
