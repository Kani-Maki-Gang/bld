use std::{
    collections::HashMap, future::Future, net::SocketAddr, path::PathBuf, pin::Pin, sync::Arc,
};

use anyhow::{anyhow, bail, Result};
use async_ssh2_lite::{AsyncSession, AsyncSftp, TokioTcpStream};
use bld_config::{path, BldConfig};
use bld_utils::sync::IntoArc;
use futures_util::{
    AsyncReadExt as FuturesUtilAsyncReadExt, AsyncWriteExt as FuturesUtilAsyncWriteExt,
};
use tokio::{
    fs::{create_dir, File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
};
use tracing::{debug, error};
use walkdir::WalkDir;

use crate::logger::LoggerSender;

type RecursiveFuture = Pin<Box<dyn Future<Output = Result<()>>>>;

pub enum SshAuthOptions<'a> {
    Keys {
        public_key: Option<&'a str>,
        private_key: &'a str,
    },
    Password {
        password: &'a str,
    },
    Agent,
}

pub struct SshConnectOptions<'a> {
    pub host: &'a str,
    pub port: u16,
    pub user: &'a str,
    pub auth: SshAuthOptions<'a>,
}

impl<'a> SshConnectOptions<'a> {
    pub fn new(host: &'a str, port: u16, user: &'a str, auth: SshAuthOptions<'a>) -> Self {
        Self {
            host,
            port,
            user,
            auth,
        }
    }
}

pub struct SshExecutionOptions<'a> {
    pub config: Arc<BldConfig>,
    pub pipeline_env: &'a HashMap<String, String>,
    pub env: Arc<HashMap<String, String>>,
}

impl<'a> SshExecutionOptions<'a> {
    pub fn new(
        config: Arc<BldConfig>,
        pipeline_env: &'a HashMap<String, String>,
        env: Arc<HashMap<String, String>>,
    ) -> Self {
        Self {
            config,
            pipeline_env,
            env,
        }
    }
}

pub struct Ssh {
    session: AsyncSession<TokioTcpStream>,
    env: HashMap<String, String>,
}

impl Ssh {
    pub async fn new<'a>(
        connect: SshConnectOptions<'a>,
        execution: SshExecutionOptions<'a>,
    ) -> Result<Self> {
        let addr: SocketAddr = format!("{}:{}", connect.host, connect.port).parse()?;
        let mut session = AsyncSession::<TokioTcpStream>::connect(addr, None).await?;
        session.handshake().await?;

        let mut instance = Self {
            session,
            env: HashMap::new(),
        };
        instance.set_auth(connect.user, &connect.auth).await?;
        instance.set_environment(execution.pipeline_env, execution.env);

        Ok(instance)
    }

    async fn set_auth<'a>(&mut self, user: &'a str, auth: &SshAuthOptions<'a>) -> Result<()> {
        match auth {
            SshAuthOptions::Agent => {
                self.session.userauth_agent(user).await?;
            }
            SshAuthOptions::Password { password } => {
                self.session.userauth_password(user, password).await?;
            }
            SshAuthOptions::Keys {
                public_key,
                private_key,
            } => {
                let public_key = public_key.map(|p| path![p]);
                let private_key = path![private_key];
                self.session
                    .userauth_pubkey_file(user, public_key.as_deref(), &private_key, None)
                    .await?;
            }
        }
        Ok(())
    }

    fn set_environment(
        &mut self,
        pipeline_env: &HashMap<String, String>,
        env: Arc<HashMap<String, String>>,
    ) {
        for (k, v) in pipeline_env.iter() {
            self.env.insert(k.to_string(), v.to_string());
        }

        for (k, v) in env.iter() {
            self.env.insert(k.to_string(), v.to_string());
        }
    }

    async fn copy_file_from(
        sftp: Arc<AsyncSftp<TokioTcpStream>>,
        from: PathBuf,
        to: PathBuf,
    ) -> Result<()> {
        debug!("fetching content of remote file {}", from.display());
        let mut remote_file = sftp.open(&from).await?;
        let mut content = String::new();
        remote_file.read_to_string(&mut content).await?;

        debug!("writing content to local file {}", to.display());
        let to = path![to];
        let mut local_file = if to.exists() {
            OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(to)
                .await?
        } else {
            File::create(to).await?
        };
        let bytes = content.as_bytes();
        local_file.write_all(bytes).await?;
        local_file.flush().await?;

        debug!("finished copying remote file from server");
        Ok(())
    }

    async fn traverse_and_copy_from_remote_directory(
        sftp: Arc<AsyncSftp<TokioTcpStream>>,
        from: PathBuf,
        to: PathBuf,
    ) -> RecursiveFuture {
        debug!(
            "starting copying contents from remote path {} to local path {}",
            from.display(),
            to.display()
        );
        Box::pin(async move {
            let remote_path = sftp.realpath(&from).await?;
            let remote_dir = sftp.readdir(&remote_path).await?;

            for (path, stat) in remote_dir.into_iter() {
                let file_name = path
                    .file_name()
                    .ok_or_else(|| anyhow!("unable to look up remote file name"))?;

                let to = path![&to, file_name];

                if stat.is_file() {
                    debug!(
                        "copying remote file {} to local path {}",
                        path.display(),
                        to.display()
                    );
                    Self::copy_file_from(sftp.clone(), path, to).await?;
                } else {
                    debug!("creating new local directory on path {}", to.display());
                    if let Err(e) = create_dir(&to).await {
                        debug!(
                            "tried to create local directory {} but encountered error {e}",
                            remote_path.display()
                        );
                    }

                    debug!("traversing remote path {}", path.display());
                    Self::traverse_and_copy_from_remote_directory(sftp.clone(), path, to)
                        .await
                        .await?;
                }
            }

            Ok(())
        })
    }

    pub async fn copy_from(&self, from: &str, to: &str) -> Result<()> {
        let sftp = self.session.sftp().await?.into_arc();
        let from = path![from];
        let to = path![to];
        let remote_path = sftp.realpath(&from).await?;

        debug!("remote path {} is a file", from.display());
        if remote_path.is_file() {
            Self::copy_file_from(sftp, from, to).await?;
            return Ok(());
        }

        debug!("creating new local directory on path {}", to.display());
        if let Err(e) = create_dir(&to).await {
            debug!(
                "tried to create local directory {} but encountered error {e}",
                remote_path.display()
            );
        }

        Self::traverse_and_copy_from_remote_directory(sftp, from, to)
            .await
            .await?;

        Ok(())
    }

    async fn copy_file_into(
        &self,
        sftp: &AsyncSftp<TokioTcpStream>,
        from: &str,
        to: &str,
    ) -> Result<()> {
        let mut local_file = File::open(&from).await?;
        let mut content = String::new();
        local_file.read_to_string(&mut content).await?;
        let bytes = content.as_bytes();

        let to = path![to];
        let mut remote_path_iter = to.iter().peekable();
        let mut remote_path = path![];
        while let Some(node) = remote_path_iter.next() {
            remote_path.push(node);
            if remote_path_iter.peek().is_none() {
                break;
            }
            if let Err(e) = sftp.mkdir(&remote_path, 0o777).await {
                debug!(
                    "tried to create remote directory {} but encountered error {e}",
                    remote_path.display()
                );
            } else {
                debug!("created remote directory {}", remote_path.display());
            }
        }

        debug!("creating target file {} using sftp", remote_path.display());
        let mut remote_file = sftp.create(&remote_path).await?;
        debug!("writing content to remote file");
        remote_file.write_all(bytes).await?;
        debug!("flushing remote file");
        remote_file.flush().await?;

        debug!("finished copying local file to server");
        Ok(())
    }

    pub async fn copy_into(&self, from: &str, to: &str) -> Result<()> {
        debug!("creating sftp channel for {to}");
        let sftp = self.session.sftp().await?;
        let from_path = path![from];

        if from_path.is_file() {
            debug!("starting copy of file {}", from_path.display());
            self.copy_file_into(&sftp, from, to).await?;
            return Ok(());
        }

        debug!("starting copy of directory {}", from_path.display());
        for dir_entry in WalkDir::new(from) {
            let Ok(dir_entry) = dir_entry.map(|e| e.into_path()).map_err(|e| error!("{e}")) else {
                continue;
            };

            if dir_entry.is_dir() {
                continue;
            }

            let to = path![to, dir_entry.strip_prefix(&from_path)?];

            debug!(
                "copying file {} to remote path {}",
                dir_entry.display(),
                to.display()
            );

            let from = dir_entry
                .to_str()
                .ok_or_else(|| anyhow!("unable to construct source path (from)"))?;

            let to = to
                .to_str()
                .ok_or_else(|| anyhow!("unable to construct destination path (to)"))?;

            self.copy_file_into(&sftp, from, to).await?;
        }
        Ok(())
    }

    pub async fn sh(
        &mut self,
        logger: Arc<LoggerSender>,
        working_dir: &Option<String>,
        input: &str,
    ) -> Result<()> {
        let mut command = String::new();
        if let Some(wd) = working_dir {
            command.push_str(&format!("cd {wd} && "));
        }
        command.push_str(input);

        let mut channel = self.session.channel_session().await?;

        for (k, v) in self.env.iter() {
            channel.setenv(k, v).await?;
        }

        channel.exec(&command).await?;

        let mut output = String::new();

        let mut stdout = String::new();
        FuturesUtilAsyncReadExt::read_to_string(&mut channel, &mut stdout).await?;
        output.push_str(&stdout);

        let mut stderr = String::new();
        let mut channel_stderr = channel.stderr();
        FuturesUtilAsyncReadExt::read_to_string(&mut channel_stderr, &mut stderr).await?;
        output.push_str(&stderr);

        logger.write(output).await?;

        let exit_status = channel.exit_status()?;
        if exit_status != 0 {
            bail!("command finished with status {exit_status}");
        }

        channel.close().await?;

        Ok(())
    }

    pub async fn dispose(&mut self) -> Result<()> {
        self.session.disconnect(None, "", None).await?;
        Ok(())
    }
}
