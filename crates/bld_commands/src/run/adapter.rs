use actix::{Actor, StreamHandler, io::SinkWrite};
use anyhow::{Result, anyhow};
use bld_config::BldConfig;
use bld_core::{context::Context, fs::FileSystem, logger::Logger};
use bld_http::{HttpClient, WebSocket};
use bld_models::dtos::ExecClientMessage;
use bld_runner::RunnerBuilder;
use bld_sock::ExecClient;
use bld_utils::sync::IntoArc;
use futures::stream::StreamExt;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::time::sleep;
use tracing::debug;

use crate::signals::CommandSignals;

pub struct LocalRun {
    config: Arc<BldConfig>,
    pipeline: String,
    inputs: HashMap<String, String>,
    env: HashMap<String, String>,
}

pub struct HttpRequest {
    config: Arc<BldConfig>,
    pipeline: String,
    inputs: HashMap<String, String>,
    env: HashMap<String, String>,
    server: String,
}

pub struct WebSocketRequest {
    config: Arc<BldConfig>,
    pipeline: String,
    inputs: HashMap<String, String>,
    env: HashMap<String, String>,
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
        inputs: HashMap<String, String>,
        env: HashMap<String, String>,
    ) -> Self {
        Self {
            config: RunConfiguration::Local(LocalRun {
                config,
                pipeline,
                inputs,
                env,
            }),
        }
    }

    pub fn server(self, server: Option<&String>) -> RunBuilder {
        match (server, self.config) {
            (None, RunConfiguration::Local(local)) => RunBuilder {
                config: RunConfiguration::Local(LocalRun {
                    config: local.config,
                    pipeline: local.pipeline,
                    inputs: local.inputs,
                    env: local.env,
                }),
            },

            (Some(server), RunConfiguration::Local(local)) => RunBuilder {
                config: RunConfiguration::WebSocket(WebSocketRequest {
                    config: local.config,
                    pipeline: local.pipeline,
                    inputs: local.inputs,
                    env: local.env,
                    server: server.to_string(),
                }),
            },

            (None, RunConfiguration::WebSocket(socket)) => RunBuilder {
                config: RunConfiguration::WebSocket(WebSocketRequest {
                    config: socket.config,
                    pipeline: socket.pipeline,
                    inputs: socket.inputs,
                    env: socket.env,
                    server: socket.server,
                }),
            },

            (Some(server), RunConfiguration::WebSocket(socket)) => RunBuilder {
                config: RunConfiguration::WebSocket(WebSocketRequest {
                    config: socket.config,
                    pipeline: socket.pipeline,
                    inputs: socket.inputs,
                    env: socket.env,
                    server: server.to_string(),
                }),
            },

            (None, RunConfiguration::Http(http)) => RunBuilder {
                config: RunConfiguration::Http(HttpRequest {
                    config: http.config,
                    pipeline: http.pipeline,
                    inputs: http.inputs,
                    env: http.env,
                    server: http.server,
                }),
            },

            (Some(server), RunConfiguration::Http(http)) => RunBuilder {
                config: RunConfiguration::Http(HttpRequest {
                    config: http.config,
                    pipeline: http.pipeline,
                    inputs: http.inputs,
                    env: http.env,
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
                    inputs: local.inputs,
                    env: local.env,
                }),
            },

            (false, RunConfiguration::WebSocket(socket)) => RunBuilder {
                config: RunConfiguration::WebSocket(WebSocketRequest {
                    config: socket.config,
                    pipeline: socket.pipeline,
                    inputs: socket.inputs,
                    env: socket.env,
                    server: socket.server,
                }),
            },

            (true, RunConfiguration::WebSocket(socket)) => RunBuilder {
                config: RunConfiguration::Http(HttpRequest {
                    config: socket.config,
                    pipeline: socket.pipeline,
                    inputs: socket.inputs,
                    env: socket.env,
                    server: socket.server,
                }),
            },

            (true, RunConfiguration::Http(http)) => RunBuilder {
                config: RunConfiguration::Http(HttpRequest {
                    config: http.config,
                    pipeline: http.pipeline,
                    inputs: http.inputs,
                    env: http.env,
                    server: http.server,
                }),
            },

            (false, RunConfiguration::Http(http)) => RunBuilder {
                config: RunConfiguration::WebSocket(WebSocketRequest {
                    config: http.config,
                    pipeline: http.pipeline,
                    inputs: http.inputs,
                    env: http.env,
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
        let (cmd_signals, signals_rx) = CommandSignals::new()?;

        let runner = RunnerBuilder::default()
            .config(mode.config.clone())
            .fs(FileSystem::local(mode.config.clone()).into_arc())
            .file(&mode.pipeline)
            .logger(Logger::shell().into_arc())
            .context(Context::local(mode.config.clone()).into_arc())
            .signals(signals_rx)
            .env(mode.env.into_arc())
            .inputs(mode.inputs.into_arc())
            .build()
            .await?;

        debug!("starting run");
        let result = runner.run().await;
        debug!("finished run");

        cmd_signals.stop().await?;
        result
    }

    async fn run_web_socket(mode: WebSocketRequest) -> Result<()> {
        let server = mode.config.server(&mode.server)?;
        let auth_path = mode.config.auth_full_path(&server.name);
        let url = format!("{}/v1/ws-exec/", server.base_url_ws());
        let data = ExecClientMessage::EnqueueRun {
            name: mode.pipeline,
            env: Some(mode.env),
            inputs: Some(mode.inputs),
        };

        let web_socket = WebSocket::new(&url)?.auth(&auth_path).await;

        let (_, framed) = web_socket
            .request()
            .connect()
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        let (sink, stream) = framed.split();
        let address = ExecClient::create(|ctx| {
            ExecClient::add_stream(stream, ctx);
            ExecClient::new(
                mode.server.to_owned(),
                Logger::shell().into_arc(),
                Context::local(mode.config.clone()).into_arc(),
                SinkWrite::new(sink, ctx),
            )
        });

        debug!("sending message to socket: {:?}", data);

        address.send(data).await.map_err(|e| anyhow!(e))?;

        while address.connected() {
            sleep(Duration::from_millis(200)).await;
        }

        Ok(())
    }

    async fn run_http(mode: HttpRequest) -> Result<()> {
        HttpClient::new(mode.config, &mode.server)?
            .run(&mode.pipeline, Some(mode.env), Some(mode.inputs))
            .await
            .map(|_| println!("pipeline has been scheduled to run"))
    }

    pub async fn run(self) -> Result<()> {
        match self.config {
            RunConfiguration::Local(run) => Self::run_local(run).await,
            RunConfiguration::Http(http) => Self::run_http(http).await,
            RunConfiguration::WebSocket(socket) => Self::run_web_socket(socket).await,
        }
    }
}
