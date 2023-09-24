use crate::command::BldCommand;
use actix_web::rt::System;
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_core::{proxies::PipelineFileSystemProxy, request::HttpClient};
use bld_runner::VersionedPipeline;
use bld_utils::sync::IntoArc;
use clap::Args;
use tracing::debug;

#[derive(Args)]
#[command(about = "Pushes the contents of a pipeline to a bld server")]
pub struct PushCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(
        short = 'p',
        long = "pipeline",
        required = true,
        help = "The name of the pipeline to push"
    )]
    pipeline: String,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server to push changes to"
    )]
    server: String,

    #[arg(
        long = "ignore-deps",
        help = "Don't include other pipeline dependencies"
    )]
    ignore_deps: bool,
}

impl PushCommand {
    async fn push(self) -> Result<()> {
        let config = BldConfig::load().await?.into_arc();
        let client = HttpClient::new(config.clone(), &self.server)?;
        let proxy = PipelineFileSystemProxy::local(config.clone()).into_arc();

        debug!(
            "running push subcommand with --server: {} and --pipeline: {}",
            self.server, self.pipeline
        );

        let mut pipelines = vec![(self.pipeline.to_owned(), proxy.read(&self.pipeline).await?)];

        if !self.ignore_deps {
            print!("Resolving dependecies...");

            let mut deps = VersionedPipeline::dependencies(
                config.clone(),
                proxy.clone(),
                self.pipeline.to_owned(),
            )
            .await
            .map(|pips| {
                println!("Done.");
                pips
            })
            .map_err(|e| {
                println!("Error. {e}");
                anyhow!("")
            })?
            .into_iter()
            .map(|(n, s)| (n, s))
            .collect();

            pipelines.append(&mut deps);
        }

        for (name, content) in pipelines.into_iter() {
            print!("Pushing {}...", name);

            client
                .push(&name, &content)
                .await
                .map(|_| println!("Done."))
                .map_err(|e| {
                    println!("Error. {e}");
                    anyhow!("")
                })?;
        }
        Ok(())
    }
}

impl BldCommand for PushCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        System::new().block_on(self.push())
    }
}
