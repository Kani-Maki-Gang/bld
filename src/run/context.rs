use crate::os::{self, OSname};
use futures_util::{StreamExt};
use shiplift::{
    ContainerOptions, Docker,
    tty::TtyChunk,
};
use std::fmt::{self, Display, Formatter};
use std::io::{self, Error, ErrorKind};
use std::process::Command;

#[derive(Clone, Debug)]
pub struct Machine;

impl Machine {
    pub fn new() -> io::Result<Self> {
        Ok(Self)
    }

    pub fn sh(&self, working_dir: &str, input: &str) -> io::Result<String> {
        let os_name = os::name();

        let (shell, mut args) = match os_name {
            OSname::Windows => ("powershell.exe", Vec::<&str>::new()),
            OSname::Linux => ("bash", vec!["-c"]),
            OSname::Mac => ("sh", vec!["-c"]),
            OSname::Unknown => return Err(Error::new(ErrorKind::Other, "Could not spawn shell")),
        };
        args.push(input);

        let process = Command::new(shell)
            .args(&args)
            .current_dir(working_dir)
            .output()?;

        let mut output = String::from_utf8_lossy(&process.stderr).to_string();
        output.push_str(&format!("\r\n{}", String::from_utf8_lossy(&process.stdout)));

        Ok(output)
    }
}

#[derive(Clone)]
pub struct Container {
    image: String,
    client: Docker,
    container_id: String,
}

impl Container {
    pub async fn new(image: &str) -> io::Result<Self> {
        let uri = match "tcp://127.0.0.1:2375".parse() {
            Ok(uri) => uri,
            Err(_) => return Err(Error::new(ErrorKind::Other, "could not parse tcp address for docker daemon"))
        };
        let docker = Docker::host(uri);
        let options = ContainerOptions::builder(&image).tty(true).build();
        let info = match docker.containers().create(&options).await {
            Ok(info) => info,
            Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
        };
        if let Err(e) = docker.containers().get(&info.id).start().await {
            return Err(Error::new(ErrorKind::Other, e.to_string()));
        }
        Ok(Self {
            image: image.to_string(),
            client: docker,
            container_id: info.id, 
        })
    }

    pub async fn sh(&mut self, _working_dir: &str, input: &str) -> io::Result<String> {
        let options = shiplift::ExecContainerOptions::builder()
            .cmd(vec![
                "bash",
                "-c",
                input
            ]) 
            .attach_stdout(true)
            .attach_stderr(true)
            .build();

        let container = self
            .client
            .containers()
            .get(&self.container_id);

        while let Some(result) = container.exec(&options).next().await {
            return match result {
                Ok(TtyChunk::StdOut(bytes)) => Ok(String::from_utf8(bytes).unwrap()),
                Ok(TtyChunk::StdErr(bytes)) => Ok(String::from_utf8(bytes).unwrap()),
                Ok(TtyChunk::StdIn(_)) => unreachable!(),
                Err(e) => Err(Error::new(ErrorKind::Other, e.to_string()))
            }; 
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
            return Err(Error::new(ErrorKind::Other, e.to_string()))
        }

        let delete_res = self
            .client
            .containers()
            .get(&self.container_id)
            .delete()
            .await;

        if let Err(e) = delete_res {
            return Err(Error::new(ErrorKind::Other, e.to_string()))
        }

        Ok(())
    }
}

pub enum RunPlatform {
    Local(Machine),
    Docker(Container),
}

impl Display for RunPlatform {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Local(_) => write!(f, "machine"),
            Self::Docker(container) => write!(f, "docker [ {} ]", container.image),
        }
    }
}
