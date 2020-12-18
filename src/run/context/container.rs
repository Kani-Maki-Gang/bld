use crate::config::BldConfig;
use crate::persist::Logger;
use crate::types::{BldError, Result};
use futures_util::StreamExt;
use shiplift::tty::TtyChunk;
use shiplift::{ContainerOptions, Docker, ExecContainerOptions, ImageListOptions, PullOptions};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

pub struct Container {
    pub img: String,
    pub client: Docker,
    pub id: String,
    pub lg: Arc<Mutex<dyn Logger>>,
}

impl Container {
    fn address() -> Result<String> {
        let config = BldConfig::load()?;
        let host = &config.local.docker_host;
        let port = &config.local.docker_port;
        Ok(format!("tcp://{}:{}", host, port))
    }

    fn docker() -> Result<Docker> {
        let uri = Container::address()?.parse()?;
        Ok(Docker::host(uri))
    }

    async fn pull(client: &Docker, image: &str, logger: &mut Arc<Mutex<dyn Logger>>) -> Result<()> {
        let options = ImageListOptions::builder().filter_name(image).build();
        let images = client.images().list(&options).await?;
        if images.len() == 0 {
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
                    logger.dumpln(&format!("{}", info.to_string()))
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
    ) -> Result<String> {
        Container::pull(client, image, logger).await?;
        let options = ContainerOptions::builder(&image).tty(true).build();
        let info = client.containers().create(&options).await?;
        client.containers().get(&info.id).start().await?;
        Ok(info.id)
    }

    pub async fn new(img: &str, mut lg: Arc<Mutex<dyn Logger>>) -> Result<Self> {
        let client = Container::docker()?;
        let id = Container::create(&client, img, &mut lg).await?;
        Ok(Self {
            img: img.to_string(),
            client,
            id,
            lg,
        })
    }

    pub async fn sh(&self, working_dir: &Option<String>, input: &str) -> Result<()> {
        let input = match working_dir {
            Some(wd) => format!("cd {} && {}", &wd, input),
            None => input.to_string(),
        };

        let cmd = vec!["bash", "-c", &input];

        let options = ExecContainerOptions::builder()
            .cmd(cmd)
            .attach_stdout(true)
            .attach_stderr(true)
            .build();

        let container = self.client.containers().get(&self.id);

        let mut exec_iter = container.exec(&options);
        while let Some(result) = exec_iter.next().await {
            let chunk = match result {
                Ok(TtyChunk::StdOut(bytes)) => String::from_utf8(bytes).unwrap(),
                Ok(TtyChunk::StdErr(bytes)) => String::from_utf8(bytes).unwrap(),
                Ok(TtyChunk::StdIn(_)) => unreachable!(),
                Err(e) => return Err(BldError::ShipliftError(e.to_string())),
            };
            {
                let mut logger = self.lg.lock().unwrap();
                logger.dump(&format!("{}", &chunk));
            }
            sleep(Duration::from_millis(100));
        }

        Ok(())
    }

    pub async fn dispose(&self) -> Result<()> {
        self.client.containers().get(&self.id).stop(None).await?;
        self.client.containers().get(&self.id).delete().await?;
        Ok(())
    }
}
