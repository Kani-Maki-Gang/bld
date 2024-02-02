use crate::command::BldCommand;
use actix_web::rt::System;
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_http::HttpClient;
use bld_utils::sync::IntoArc;
use clap::Args;
use tracing::debug;

#[derive(Args)]
#[command(about = "Pull a pipeline from a bld server and stores it localy")]
pub struct PullCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
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
    server: String,

    #[arg(
        long = "ignore-deps",
        help = "Do not include other pipeline dependencies"
    )]
    ignore_deps: bool,
}

impl PullCommand {
    async fn request(self) -> Result<()> {
        let config = BldConfig::load().await?.into_arc();
        let client = HttpClient::new(config.clone(), &self.server)?;
        let proxy = PipelineFileSystemProxy::local(config);

        debug!(
            "running pull subcommand with --server: {}, --pipeline: {} and --ignore-deps: {}",
            self.server, self.pipeline, self.ignore_deps
        );

        let mut pipelines = vec![self.pipeline.to_string()];

        if !self.ignore_deps {
            print!("Fetching metadata for dependecies...");

            let mut deps = client
                .deps(&self.pipeline)
                .await
                .map(|deps| {
                    println!("Done.");
                    deps
                })
                .map_err(|e| {
                    println!("Error. {e}");
                    anyhow!(String::new())
                })?;

            pipelines.append(&mut deps);
        }

        for pipeline in pipelines.iter() {
            print!("Pulling pipeline {pipeline}...");

            let data = client
                .pull(pipeline)
                .await
                .map(|data| {
                    println!("Done.");
                    data
                })
                .map_err(|err| {
                    println!("Error. {err}");
                    err
                })?;

            proxy.create(&data.name, &data.content, true).await?;
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
