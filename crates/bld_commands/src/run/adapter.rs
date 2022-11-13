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
use std::fmt::Write;
use std::sync::Arc;
use tracing::debug;

// pub trait RunAdapter {
//     fn start(self) -> Result<()>;
// }
//
// struct RunInProcessAdapter {
//     config: Arc<BldConfig>,
//     pipeline: String,
//     environment: HashMap<String, String>,
//     variables: HashMap<String, String>,
// }
//
// impl RunAdapter for RunInProcessAdapter {
//     fn start(self) -> Result<()> {
//         let sys = System::new();
//
//         let res = sys.block_on(async move {
//             let runner = RunnerBuilder::default()
//                 .config(self.config.clone())
//                 .pipeline(&self.pipeline)
//                 .logger(LoggerSender::shell().into_arc())
//                 .environment(self.environment.clone().into_arc())
//                 .variables(self.variables.clone().into_arc())
//                 .build()
//                 .await?;
//             let res = runner.run().await.await;
//
//             System::current().stop();
//             res
//         });
//
//         sys.run()?;
//         debug!("local run finished");
//         res
//     }
// }
//
// struct RunWithHttpRequestAdapter {
//     request: Request,
//     pipeline: String,
//     environment: HashMap<String, String>,
//     variables: HashMap<String, String>,
// }
//
// impl RunWithHttpRequestAdapter {
//     async fn send_request(self) -> Result<()> {
//         let data = RunInfo::new(
//             &self.pipeline,
//             Some(self.environment.clone()),
//             Some(self.variables.clone()),
//         );
//         self.request
//             .send_json(data)
//             .await
//             .map(|_: String| {
//                 println!("pipeline has been scheduled to run");
//             })
//     }
// }
//
// impl RunAdapter for RunWithHttpRequestAdapter {
//     fn start(self) -> Result<()> {
//         System::new().block_on(async move { self.send_request().await })
//     }
// }
//
// struct RunWithWebSocketAdapter {
//     web_socket: WebSocket,
//     pipeline: String,
//     environment: HashMap<String, String>,
//     variables: HashMap<String, String>,
// }
//
// impl RunWithWebSocketAdapter {
//     async fn connect_to_socket(self) -> Result<()> {
//         let (_, framed) = self
//             .web_socket
//             .request()
//             .connect()
//             .await
//             .map_err(|e| anyhow!(e.to_string()))?;
//
//         let (sink, stream) = framed.split();
//         let addr = ExecClient::create(|ctx| {
//             ExecClient::add_stream(stream, ctx);
//             ExecClient::new(LoggerSender::shell().into_arc(), SinkWrite::new(sink, ctx))
//         });
//
//         debug!(
//             "sending self over: {:?} {:?}",
//             self.pipeline, self.variables
//         );
//
//         addr.send(RunInfo::new(
//             &self.pipeline,
//             Some(self.environment.clone()),
//             Some(self.variables.clone()),
//         ))
//         .await
//         .map_err(|e| anyhow!(e))
//     }
// }
//
// impl RunAdapter for RunWithWebSocketAdapter {
//     fn start(self) -> Result<()> {
//         let sys = System::new();
//         let res = sys.block_on(async move { self.connect_to_socket().await });
//         sys.run()?;
//         debug!("server run finished");
//         res
//     }
// }
//
// struct ServerProperties {
//     pub host: String,
//     pub port: i64,
//     pub protocol: String,
// }

pub trait RunConfiguration { }

pub struct Local {
    config: Arc<BldConfig>,
    pipeline: String,
    variables: HashMap<String, String>,
    environment: HashMap<String, String>,
}

impl RunConfiguration for Local { }

pub struct HttpRequest {
    config: Arc<BldConfig>,
    pipeline: String,
    variables: HashMap<String, String>,
    environment: HashMap<String, String>,
    server: String,
}

impl RunConfiguration for HttpRequest { }

pub struct WebSocketRequest {
    config: Arc<BldConfig>,
    pipeline: String,
    variables: HashMap<String, String>,
    environment: HashMap<String, String>,
    server: String,
}

impl RunConfiguration for WebSocketRequest { }

pub struct RunBuilder<T: RunConfiguration> {
    mode: T,
}

impl RunBuilder<Local> {
    pub fn new(
        config: Arc<BldConfig>,
        pipeline: String,
        variables: HashMap<String, String>,
        environment: HashMap<String, String>,
    ) -> Self {
        Self {
            mode: Local {
                config,
                pipeline,
                variables,
                environment,
            }
        }
    }

    pub fn server(self, server: &str) -> RunBuilder<WebSocketRequest> {
        RunBuilder {
            mode: WebSocketRequest {
                config: self.mode.config,
                pipeline: self.mode.pipeline,
                variables: self.mode.variables,
                environment: self.mode.environment,
                server: server.to_string()
            },
        }
    }

    pub fn build(self) -> RunAdapter<Local> {
        RunAdapter {
            mode: self.mode
        }
    }
}

impl RunBuilder<WebSocketRequest> {
    pub fn detach(self) -> RunBuilder<T> {
        RunBuilder {
            mode: HttpRequest {
                config: self.mode.config,
                pipeline: self.mode.pipeline,
                variables: self.mode.variables,
                environment: self.mode.environment,
                server: self.mode.server,
            },
        }
    }

    pub fn build(self) -> RunAdapter<WebSocketRequest> {
        RunAdapter {
            mode: self.mode
        }
    }
}

impl RunBuilder<HttpRequest> {
    pub fn build(self) -> RunAdapter<HttpRequest> {
        RunAdapter {
            mode: self.mode
        }
    }
}

struct RunAdapter<Local> {
    mode: Local
}

impl RunAdapter<Local> {
    pub fn run(self) -> Result<()> {
        let runner_builder = RunnerBuilder::default()
            .config(self.mode.config)
            .pipeline(&self.mode.pipeline)
            .logger(LoggerSender::shell().into_arc())
            .environment(self.mode.environment.into_arc())
            .variables(self.mode.variables.into_arc());

        let system = System::new();
        let result = system.block_on(async move {
            let runner = runner_builder
                .build()
                .await?;
            let result = runner.run().await.await;

            System::current().stop();
            result
        });

        system.run()?;
        debug!("local run finished");
        result
    }
}

impl RunAdapter<HttpRequest> {
    pub fn run(self) -> Result<()> {
        let server = self.mode.config.remote.server(&self.mode.server)?;
        let server_auth = self.mode.config.remote.same_auth_as(&server)?;

        let url = format!(
            "{}://{}:{}",
            server.http_protocol(), server.host, server.port
        );

        let data = RunInfo::new(
            &self.mode.pipeline,
            Some(self.mode.environment.clone()),
            Some(self.mode.variables.clone()),
        );

        let request = Request::post(&url).auth(&server_auth);

        System::new().block_on(async move {
            request
                .send_json(data)
                .await
                .map(|_: String| {
                    println!("pipeline has been scheduled to run");
                })
        })
    }
}

impl RunAdapter<WebSocketRequest> {
    pub fn run(self) -> Result<()> {
        let server = self.mode.config.remote.server(&self.mode.server)?;
        let server_auth = self.mode.config.remote.same_auth_as(&server)?;

        let url = format!(
            "{}://{}:{}",
            server.ws_protocol(), server.host, server.port
        );

        let data = RunInfo::new(
            &self.mode.pipeline,
            Some(self.mode.environment),
            Some(self.mode.variables),
        );

        let web_socket = WebSocket::new(&url)?.auth(&server_auth);

        let system = System::new();
        let result = system.block_on(async move {
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

            address
                .send(data)
                .await
                .map_err(|e| anyhow!(e))
        });

        system.run()?;
        result
    }
}

// pub fn create_adapter(
//     config: BldConfig,
//     pipeline: String,
//     server: Option<&String>,
//     vars: HashMap<String, String>,
//     env: HashMap<String, String>,
//     detach: bool,
// ) -> Result<Box<dyn RunAdapter>> {
//     if let Some(server) = server {
//         let server = config.remote.server(server)?;
//         let server_auth = config.remote.same_auth_as(server)?;
//         let protocol = if detach {
//             server.http_protocol()
//         } else {
//             server.ws_protocol()
//         };
//         let mut url = format!(
//             "{}://{}:{}",
//             protocol, server.host, server.port
//         );
//
//         if detach {
//             write!(url, "/run");
//             let request = Request::post(&url).auth(&server_auth);
//
//             Ok(Box::new(RunWithHttpRequestAdapter {
//                 request,
//                 pipeline,
//                 environment: env,
//                 variables: vars,
//             }))
//         } else {
//             write!(url, "/ws-exec/");
//             let web_socket = WebSocket::new(&url)?.auth(&server_auth);
//
//             Ok(Box::new(RunWithWebSocketAdapter {
//                 web_socket,
//                 pipeline,
//                 environment: env,
//                 variables: vars,
//             }))
//         }
//     } else {
//         Ok(Box::new(RunInProcessAdapter {
//             config: config.into_arc(),
//             pipeline,
//             environment: env,
//             variables: vars,
//         }))
//     }
// }
