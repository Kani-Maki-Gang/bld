use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};

use anyhow::{bail, Result};
use async_ssh2_lite::{AsyncSession, TokioTcpStream};
use bld_config::{path, BldConfig};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::logger::LoggerSender;

pub struct SshConnectOptions<'a> {
    pub host: &'a str,
    pub port: u16,
    pub user: &'a str,
    pub public_key: &'a str,
    pub private_key: &'a str,
}

impl<'a> SshConnectOptions<'a> {
    pub fn new(
        host: &'a str,
        port: u16,
        user: &'a str,
        public_key: &'a str,
        private_key: &'a str,
    ) -> Self {
        Self {
            host,
            port,
            user,
            public_key,
            private_key,
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

        let public_key = path![connect.public_key];
        let private_key = path![connect.private_key];
        session
            .userauth_pubkey_file(connect.user, Some(&public_key), &private_key, None)
            .await?;

        let mut instance = Self { session, env: HashMap::new() };
        instance.set_environment(execution.pipeline_env, execution.env);

        Ok(instance)
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

    pub async fn copy_from(&self, from: &str, to: &str) -> Result<()> {
        let path = path![from];
        let (mut scp_channel, _scp_stat) = self.session.scp_recv(&path).await?;
        let mut content = String::new();
        scp_channel.read_to_string(&mut content).await?;

        let bytes = content.as_bytes();
        File::create(to).await?.write_all(&bytes).await?;

        Ok(())
    }

    pub async fn copy_into(&self, from: &str, to: &str) -> Result<()> {
        let from = path![from];

        if from.is_dir() {
            bail!("unable to transfer an entire directory");
        }

        let to = path![to];
        let mut file = File::open(&from).await?;
        let metadata = file.metadata().await?;
        let size = metadata.len();
        let mut scp_channel = self.session.scp_send(&to, 777, size, None).await?;

        let mut content = String::new();
        file.read_to_string(&mut content).await?;
        let bytes = content.as_bytes();
        scp_channel.write_all(&bytes).await?;

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
        channel.read_to_string(&mut output).await?;

        logger.write(output).await?;

        channel.close().await?;

        Ok(())
    }

    pub async fn dispose(&mut self) -> Result<()> {
        self.session.disconnect(None, "", None).await?;
        Ok(())
    }
}
