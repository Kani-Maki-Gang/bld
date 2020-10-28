use crate::config::BldConfig;
use crate::persist::Dumpster;
use futures_util::StreamExt;
use shiplift::{
    tty::TtyChunk, ContainerOptions, Docker, ExecContainerOptions, ImageListOptions, PullOptions,
};
use std::io::{self, Error, ErrorKind};
use std::thread::sleep;
use std::time::Duration;
use std::sync::{Arc, Mutex};

pub struct Container {
    pub image: String,
    pub client: Docker,
    pub container_id: String,
    pub dumpster: Arc<Mutex<dyn Dumpster>>,
}

impl Container {
    fn address() -> io::Result<String> {
        let config = BldConfig::load()?;
        let host = &config.local.docker_host;
        let port = &config.local.docker_port;
        Ok(format!("tcp://{}:{}", host, port))
    }

    fn docker() -> io::Result<Docker> {
        let uri = match (Container::address()?).parse() {
            Ok(uri) => uri,
            Err(_) => {
                return Err(Error::new(
                    ErrorKind::Other,
                    "could not parse tcp address for docker daemon",
                ))
            }
        };
        Ok(Docker::host(uri))
    }

    async fn pull(client: &Docker, image: &str, dumpster: &mut Arc<Mutex<dyn Dumpster>>) -> io::Result<()> {
        let options = ImageListOptions::builder().filter_name(image).build();
        let images = match client.images().list(&options).await {
            Ok(img) => img,
            Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
        };

        if images.len() == 0 {
            {
                let mut dumpster = dumpster.lock().unwrap();
                dumpster.info(&format!("Download image: {}", image));
            }

            let options = PullOptions::builder().image(image).build();
            let mut pull_iter = client.images().pull(&options);
            while let Some(progress) = pull_iter.next().await {
                match progress {
                    Ok(info) => {
                        let mut dumpster = dumpster.lock().unwrap();
                        dumpster.dumpln(&format!("{}", info.to_string()))
                    },
                    Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
                }
                sleep(Duration::from_millis(100));
            }
        }

        Ok(())
    }

    async fn create(client: &Docker, image: &str, dumpster: &mut Arc<Mutex<dyn Dumpster>>) -> io::Result<String> {
        Container::pull(client, image, dumpster).await?;

        let options = ContainerOptions::builder(&image).tty(true).build();
        let info = match client.containers().create(&options).await {
            Ok(info) => info,
            Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
        };

        if let Err(e) = client.containers().get(&info.id).start().await {
            return Err(Error::new(ErrorKind::Other, e.to_string()));
        }

        Ok(info.id)
    }

    pub async fn new(image: &str, mut dumpster: Arc<Mutex<dyn Dumpster>>) -> io::Result<Self> {
        let client = Container::docker()?;
        let container_id = Container::create(&client, image, &mut dumpster).await?;

        Ok(Self {
            image: image.to_string(),
            client,
            container_id,
            dumpster,
        })
    }

    pub async fn sh(&self, working_dir: &Option<String>, input: &str) -> io::Result<String> {
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

        let container = self.client.containers().get(&self.container_id);

        let mut exec_iter = container.exec(&options);
        while let Some(result) = exec_iter.next().await {
            let chunk = match result {
                Ok(TtyChunk::StdOut(bytes)) => String::from_utf8(bytes).unwrap(),
                Ok(TtyChunk::StdErr(bytes)) => String::from_utf8(bytes).unwrap(),
                Ok(TtyChunk::StdIn(_)) => unreachable!(),
                Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
            };
            {
                let mut dumpster = self.dumpster.lock().unwrap();
                dumpster.dump(&format!("{}", &chunk));
            }
            sleep(Duration::from_millis(100));
        }

        Ok(String::new())
    }

    pub async fn dispose(&self) -> io::Result<()> {
        let stop_res = self
            .client
            .containers()
            .get(&self.container_id)
            .stop(None)
            .await;

        if let Err(e) = stop_res {
            return Err(Error::new(ErrorKind::Other, e.to_string()));
        }

        let delete_res = self
            .client
            .containers()
            .get(&self.container_id)
            .delete()
            .await;

        if let Err(e) = delete_res {
            return Err(Error::new(ErrorKind::Other, e.to_string()));
        }

        Ok(())
    }
}
