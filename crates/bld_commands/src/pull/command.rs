use crate::command::BldCommand;
use actix_web::rt::System;
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_server::responses::PullResponse;
use bld_utils::request::Request;
use bld_utils::sync::IntoArc;
use clap::Args;
use tracing::debug;

#[derive(Args)]
#[command(about = "Pull a pipeline from a bld server and stores it localy")]
pub struct PullCommand {
    #[arg(short = 'v', long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(
        short = 'p',
        long = "pipeline",
        required = true,
        help = "The name of the bld server"
    )]
    pipeline: String,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server to pull the pipeline from"
    )]
    server: Option<String>,

    #[arg(
        long = "ignore-deps",
        help = "Do not include other pipeline dependencies"
    )]
    ignore_deps: bool,
}

impl PullCommand {
    async fn request(self) -> Result<()> {
        let config = BldConfig::load()?.into_arc();
        let proxy = PipelineFileSystemProxy::local(config.clone());
        let server = config.server_or_first(self.server.as_ref())?;
        let server_auth = config.same_auth_as(server)?;

        let base_url = server.base_url_http();
        let metadata_url = format!("{}/deps", base_url);
        let url = format!("{}/pull", base_url);

        debug!(
            "running pull subcommand with --server: {}, --pipeline: {} and --ignore-deps: {}",
            server.name, self.pipeline, self.ignore_deps
        );

        let mut pipelines = vec![self.pipeline.to_string()];

        if !self.ignore_deps {
            debug!("sending http request to {}", metadata_url);
            print!("Fetching metadata for dependecies...");

            Request::post(&metadata_url)
                .auth(server_auth)
                .send_json(&self.pipeline)
                .await
                .map(|mut deps: Vec<String>| {
                    println!("Done.");
                    pipelines.append(&mut deps);
                })
                .map_err(|e| {
                    println!("Error. {e}");
                    anyhow!(String::new())
                })?;
        }

        for pipeline in pipelines.iter() {
            debug!("sending http request to {}", url);
            print!("Pulling pipeline {pipeline}...");

            let data: PullResponse = Request::post(&url)
                .auth(server_auth)
                .send_json(pipeline)
                .await
                .map(|data| {
                    println!("Done.");
                    data
                })
                .map_err(|err| {
                    println!("Error. {err}");
                    err
                })?;

            proxy.create(&data.name, &data.content, true)?;
        }

        Ok(())
    }
}

impl BldCommand for PullCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        System::new().block_on(self.request())
    }
}
