use actix::{io::SinkWrite, Actor, StreamHandler};
use actix_web::rt::System;
use anyhow::{anyhow, Result};
use awc::http::Version;
use awc::Client;
use bld_config::BldConfig;
use bld_core::logger::Logger;
use bld_runner::RunnerBuilder;
use bld_server::requests::RunInfo;
use bld_server::sockets::ExecClient;
use bld_utils::request::{self, headers};
use futures::stream::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tracing::debug;

struct ServerProperties {
    pub host: String,
    pub port: i64,
    pub protocol: String,
    pub headers: HashMap<String, String>,
}

pub struct InvokeRun {
    config: Arc<BldConfig>,
    server: Option<ServerProperties>,
    detach: bool,
    pipeline: String,
    environment: HashMap<String, String>,
    variables: HashMap<String, String>,
}

impl InvokeRun {
    pub fn new(
        config: BldConfig,
        pipeline: String,
        server: Option<&str>,
        vars: HashMap<String, String>,
        env: HashMap<String, String>,
        detach: bool,
    ) -> Result<Self> {
        let mut server_props = None;
        if let Some(server) = server {
            let server = config.remote.server(server)?;
            let server_auth = config.remote.same_auth_as(server)?;
            server_props = Some(ServerProperties {
                host: server.host.clone(),
                port: server.port,
                protocol: if detach {
                    server.http_protocol()
                } else {
                    server.ws_protocol()
                },
                headers: headers(&server_auth.name, &server_auth.auth)?,
            });
        }
        Ok(Self {
            config: Arc::new(config),
            server: server_props,
            detach,
            pipeline,
            environment: env,
            variables: vars,
        })
    }

    pub fn start(&self) -> Result<()> {
        match &self.server {
            Some(_) => self.invoke_server(),
            None => self.invoke_local(),
        }
    }

    fn invoke_local(&self) -> Result<()> {
        let rt = Runtime::new()?;
        rt.block_on(async {
            let runner = RunnerBuilder::default()
                .config(self.config.clone())
                .pipeline(&self.pipeline)
                .logger(Logger::shell_atom())
                .environment(Arc::new(self.environment.clone()))
                .variables(Arc::new(self.variables.clone()))
                .build()
                .await?;
            runner.run().await.await
        })
    }

    fn invoke_server(&self) -> Result<()> {
        debug!("spawing actix system");
        if self.detach {
            System::new().block_on(async move { self.send_run_request().await })
        } else {
            let sys = System::new();
            let res = sys.block_on(async move { self.connect_to_exec_socket().await });
            sys.run()?;
            res
        }
    }

    async fn send_run_request(&self) -> Result<()> {
        let server_props = self
            .server
            .as_ref()
            .ok_or_else(|| anyhow!("no server properties"))?;

        let url = format!(
            "{}://{}:{}/run",
            server_props.protocol, server_props.host, server_props.port
        );

        debug!("sending request to {url}");

        let request_data = RunInfo::new(
            &self.pipeline,
            Some(self.environment.clone()),
            Some(self.variables.clone()),
        );
        request::post(url, server_props.headers.clone(), request_data)
            .await
            .map(|_| {
                println!("pipeline has been scheduled to run");
            })
    }

    async fn connect_to_exec_socket(&self) -> Result<()> {
        let server_props = self
            .server
            .as_ref()
            .ok_or_else(|| anyhow!("no server properties"))?;

        let url = format!(
            "{}://{}:{}/ws-exec/",
            server_props.protocol, server_props.host, server_props.port
        );

        debug!("establishing web socker connection on {}", url);

        let client = Client::builder()
            .max_http_version(Version::HTTP_11)
            .finish();
        let mut client = client.ws(url);
        for (key, value) in server_props.headers.iter() {
            client = client.header(&key[..], &value[..]);
        }

        let (_, framed) = client.connect().await.map_err(|e| anyhow!(e.to_string()))?;
        let (sink, stream) = framed.split();
        let addr = ExecClient::create(|ctx| {
            ExecClient::add_stream(stream, ctx);
            ExecClient::new(SinkWrite::new(sink, ctx))
        });

        debug!(
            "sending self over: {:?} {:?}",
            self.pipeline, self.variables
        );

        addr.send(RunInfo::new(
            &self.pipeline,
            Some(self.environment.clone()),
            Some(self.variables.clone()),
        ))
        .await
        .map_err(|e| anyhow!(e))
    }
}
