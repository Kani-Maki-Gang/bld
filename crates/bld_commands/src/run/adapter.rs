use actix::{io::SinkWrite, Actor, StreamHandler};
use actix_web::rt::System;
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_core::logger::LoggerSender;
use bld_runner::RunnerBuilder;
use bld_sock::clients::ExecClient;
use bld_sock::messages::RunInfo;
use bld_utils::request::{Request, WebSocket};
use bld_utils::sync::IntoArc;
use futures::stream::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;

pub struct LocalRun {
    config: Arc<BldConfig>,
    pipeline: String,
    variables: HashMap<String, String>,
    environment: HashMap<String, String>,
}

pub struct HttpRequest {
    config: Arc<BldConfig>,
    pipeline: String,
    variables: HashMap<String, String>,
    environment: HashMap<String, String>,
    server: String,
}

pub struct WebSocketRequest {
    config: Arc<BldConfig>,
    pipeline: String,
    variables: HashMap<String, String>,
    environment: HashMap<String, String>,
    server: String,
}

pub enum RunConfiguration {
    Local(LocalRun),
    Http(HttpRequest),
    WebSocket(WebSocketRequest),
}

pub struct RunBuilder {
    config: RunConfiguration,
}

impl RunBuilder {
    pub fn new(
        config: Arc<BldConfig>,
        pipeline: String,
        variables: HashMap<String, String>,
        environment: HashMap<String, String>,
    ) -> Self {
        Self {
            config: RunConfiguration::Local(LocalRun {
                config,
                pipeline,
                variables,
                environment,
            }),
        }
    }

    pub fn server(self, server: Option<&String>) -> RunBuilder {
        match (server, self.config) {
            (None, RunConfiguration::Local(local)) => RunBuilder {
                config: RunConfiguration::Local(LocalRun {
                    config: local.config,
                    pipeline: local.pipeline,
                    variables: local.variables,
                    environment: local.environment,
                }),
            },

            (Some(server), RunConfiguration::Local(local)) => RunBuilder {
                config: RunConfiguration::WebSocket(WebSocketRequest {
                    config: local.config,
                    pipeline: local.pipeline,
                    variables: local.variables,
                    environment: local.environment,
                    server: server.to_string(),
                }),
            },

            (None, RunConfiguration::WebSocket(socket)) => RunBuilder {
                config: RunConfiguration::WebSocket(WebSocketRequest {
                    config: socket.config,
                    pipeline: socket.pipeline,
                    variables: socket.variables,
                    environment: socket.environment,
                    server: socket.server,
                }),
            },

            (Some(server), RunConfiguration::WebSocket(socket)) => RunBuilder {
                config: RunConfiguration::WebSocket(WebSocketRequest {
                    config: socket.config,
                    pipeline: socket.pipeline,
                    variables: socket.variables,
                    environment: socket.environment,
                    server: server.to_string(),
                }),
            },

            (None, RunConfiguration::Http(http)) => RunBuilder {
                config: RunConfiguration::Http(HttpRequest {
                    config: http.config,
                    pipeline: http.pipeline,
                    variables: http.variables,
                    environment: http.environment,
                    server: http.server,
                }),
            },

            (Some(server), RunConfiguration::Http(http)) => RunBuilder {
                config: RunConfiguration::Http(HttpRequest {
                    config: http.config,
                    pipeline: http.pipeline,
                    variables: http.variables,
                    environment: http.environment,
                    server: server.to_string(),
                }),
            },
        }
    }

    pub fn detach(self, detach: bool) -> Self {
        match (detach, self.config) {
            (_, RunConfiguration::Local(local)) => RunBuilder {
                config: RunConfiguration::Local(LocalRun {
                    config: local.config,
                    pipeline: local.pipeline,
                    variables: local.variables,
                    environment: local.environment,
                }),
            },

            (false, RunConfiguration::WebSocket(socket)) => RunBuilder {
                config: RunConfiguration::WebSocket(WebSocketRequest {
                    config: socket.config,
                    pipeline: socket.pipeline,
                    variables: socket.variables,
                    environment: socket.environment,
                    server: socket.server,
                }),
            },

            (true, RunConfiguration::WebSocket(socket)) => RunBuilder {
                config: RunConfiguration::Http(HttpRequest {
                    config: socket.config,
                    pipeline: socket.pipeline,
                    variables: socket.variables,
                    environment: socket.environment,
                    server: socket.server,
                }),
            },

            (true, RunConfiguration::Http(http)) => RunBuilder {
                config: RunConfiguration::Http(HttpRequest {
                    config: http.config,
                    pipeline: http.pipeline,
                    variables: http.variables,
                    environment: http.environment,
                    server: http.server,
                }),
            },

            (false, RunConfiguration::Http(http)) => RunBuilder {
                config: RunConfiguration::WebSocket(WebSocketRequest {
                    config: http.config,
                    pipeline: http.pipeline,
                    variables: http.variables,
                    environment: http.environment,
                    server: http.server,
                }),
            },
        }
    }

    pub fn build(self) -> RunAdapter {
        RunAdapter {
            config: self.config,
        }
    }
}

pub struct RunAdapter {
    config: RunConfiguration,
}

impl RunAdapter {
    async fn run_local(mode: LocalRun) -> Result<()> {
        let runner = RunnerBuilder::default()
            .config(mode.config)
            .pipeline(&mode.pipeline)
            .logger(LoggerSender::shell().into_arc())
            .environment(mode.environment.into_arc())
            .variables(mode.variables.into_arc())
            .build()
            .await?;

        let result = runner.run().await;

        System::current().stop();
        result
    }

    async fn run_web_socket(mode: WebSocketRequest) -> Result<()> {
        let server = mode.config.server(&mode.server)?;
        let server_auth = mode.config.same_auth_as(server)?;

        let url = format!(
            "{}://{}:{}/ws-exec/",
            server.ws_protocol(),
            server.host,
            server.port
        );

        let data = RunInfo::new(&mode.pipeline, Some(mode.environment), Some(mode.variables));

        let web_socket = WebSocket::new(&url)?.auth(server_auth);

        let (_, framed) = web_socket
            .request()
            .connect()
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        let (sink, stream) = framed.split();
        let address = ExecClient::create(|ctx| {
            ExecClient::add_stream(stream, ctx);
            ExecClient::new(LoggerSender::shell().into_arc(), SinkWrite::new(sink, ctx))
        });

        debug!(
            "sending self over: {:?} {:?} {:?}",
            data.name, data.variables, data.environment
        );

        address.send(data).await.map_err(|e| anyhow!(e))
    }

    async fn run_http(mode: HttpRequest) -> Result<()> {
        let server = mode.config.server(&mode.server)?;
        let server_auth = mode.config.same_auth_as(server)?;

        let url = format!(
            "{}://{}:{}/run",
            server.http_protocol(),
            server.host,
            server.port
        );

        let data = RunInfo::new(
            &mode.pipeline,
            Some(mode.environment.clone()),
            Some(mode.variables.clone()),
        );

        let result = Request::post(&url)
            .auth(server_auth)
            .send_json(data)
            .await
            .map(|_: String| {
                println!("pipeline has been scheduled to run");
            });

        System::current().stop();
        result
    }

    pub fn run(self) -> Result<()> {
        let system = System::new();

        let result = system.block_on(async move {
            match self.config {
                RunConfiguration::Local(run) => Self::run_local(run).await,
                RunConfiguration::Http(http) => Self::run_http(http).await,
                RunConfiguration::WebSocket(socket) => Self::run_web_socket(socket).await,
            }
        });

        system.run()?;
        result
    }
}
