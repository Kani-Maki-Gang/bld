use actix::{io::SinkWrite, Actor, StreamHandler};
use actix_web::rt::System;
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_core::logger::LoggerSender;
use bld_runner::RunnerBuilder;
use bld_sock::clients::ExecClient;
use bld_sock::messages::RunInfo;
use bld_utils::request::{self, headers};
use bld_utils::sync::AsArc;
use bld_utils::tls::awc_client;
use futures::stream::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;

pub trait RunAdapter {
    fn start(&self) -> Result<()>;
}

struct RunInProcessAdapter {
    config: Arc<BldConfig>,
    pipeline: String,
    environment: HashMap<String, String>,
    variables: HashMap<String, String>,
}

impl RunAdapter for RunInProcessAdapter {
    fn start(&self) -> Result<()> {
        let sys = System::new();

        let res = sys.block_on(async move {
            let runner = RunnerBuilder::default()
                .config(self.config.clone())
                .pipeline(&self.pipeline)
                .logger(LoggerSender::shell().as_arc())
                .environment(self.environment.clone().as_arc())
                .variables(self.variables.clone().as_arc())
                .build()
                .await?;
            let res = runner.run().await.await;

            System::current().stop();
            res
        });

        sys.run()?;
        debug!("local run finished");
        res
    }
}

struct RunWithHttpRequestAdapter {
    server: Option<ServerProperties>,
    pipeline: String,
    environment: HashMap<String, String>,
    variables: HashMap<String, String>,
}

impl RunWithHttpRequestAdapter {
    async fn send_request(&self) -> Result<()> {
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
}

impl RunAdapter for RunWithHttpRequestAdapter {
    fn start(&self) -> Result<()> {
        System::new().block_on(async move { self.send_request().await })
    }
}

struct RunWithWebSocketAdapter {
    server: Option<ServerProperties>,
    pipeline: String,
    environment: HashMap<String, String>,
    variables: HashMap<String, String>,
}

impl RunWithWebSocketAdapter {
    async fn connect_to_socket(&self) -> Result<()> {
        let server_props = self
            .server
            .as_ref()
            .ok_or_else(|| anyhow!("no server properties"))?;

        let url = format!(
            "{}://{}:{}/ws-exec/",
            server_props.protocol, server_props.host, server_props.port
        );

        debug!("establishing web socker connection on {}", url);

        let mut client = awc_client()?.ws(url);
        for (key, value) in server_props.headers.iter() {
            client = client.header(&key[..], &value[..]);
        }

        let (_, framed) = client.connect().await.map_err(|e| anyhow!(e.to_string()))?;
        let (sink, stream) = framed.split();
        let addr = ExecClient::create(|ctx| {
            ExecClient::add_stream(stream, ctx);
            ExecClient::new(LoggerSender::shell().as_arc(), SinkWrite::new(sink, ctx))
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

impl RunAdapter for RunWithWebSocketAdapter {
    fn start(&self) -> Result<()> {
        let sys = System::new();
        let res = sys.block_on(async move { self.connect_to_socket().await });
        sys.run()?;
        debug!("server run finished");
        res
    }
}

struct ServerProperties {
    pub host: String,
    pub port: i64,
    pub protocol: String,
    pub headers: HashMap<String, String>,
}

pub fn create_adapter(
    config: BldConfig,
    pipeline: String,
    server: Option<&String>,
    vars: HashMap<String, String>,
    env: HashMap<String, String>,
    detach: bool,
) -> Result<Box<dyn RunAdapter>> {
    if let Some(server) = server {
        let server = config.remote.server(server)?;
        let server_auth = config.remote.same_auth_as(server)?;
        let server_props = Some(ServerProperties {
            host: server.host.clone(),
            port: server.port,
            protocol: if detach {
                server.http_protocol()
            } else {
                server.ws_protocol()
            },
            headers: headers(&server_auth.name, &server_auth.auth)?,
        });

        if detach {
            Ok(Box::new(RunWithHttpRequestAdapter {
                server: server_props,
                pipeline,
                environment: env,
                variables: vars,
            }))
        } else {
            Ok(Box::new(RunWithWebSocketAdapter {
                server: server_props,
                pipeline,
                environment: env,
                variables: vars,
            }))
        }
    } else {
        Ok(Box::new(RunInProcessAdapter {
            config: config.as_arc(),
            pipeline,
            environment: env,
            variables: vars,
        }))
    }
}
