use crate::command::BldCommand;
use actix_web::rt::System;
use anyhow::{Result, anyhow};
use bld_config::BldConfig;
use bld_core::fs::FileSystem;
use bld_http::HttpClient;
use bld_utils::sync::IntoArc;
use clap::Args;
use tracing::debug;

#[derive(Args)]
#[command(about = "Pull a file from a bld server and stores it locally")]
pub struct PullCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(required = true, help = "The name of the bld server")]
    file: String,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server to pull the file from"
    )]
    server: String,

    #[arg(long = "ignore-deps", help = "Do not include other file dependencies")]
    ignore_deps: bool,
}

impl PullCommand {
    async fn request(self) -> Result<()> {
        let config = BldConfig::load().await?.into_arc();
        let client = HttpClient::new(config.clone(), &self.server)?;
        let fs = FileSystem::local(config);

        debug!(
            "running pull subcommand with --server: {}, file: {} and --ignore-deps: {}",
            self.server, self.file, self.ignore_deps
        );

        let mut pipelines = vec![self.file.to_string()];

        if !self.ignore_deps {
            print!("Fetching metadata for dependecies...");

            let mut deps = client
                .deps(&self.file)
                .await
                .inspect(|_| {
                    println!("Done.");
                })
                .map_err(|e| {
                    println!("Error. {e}");
                    anyhow!(String::new())
                })?;

            pipelines.append(&mut deps);
        }

        for pipeline in pipelines.iter() {
            print!("Pulling file {pipeline}...");

            let data = client
                .pull(pipeline)
                .await
                .inspect(|_| {
                    println!("Done.");
                })
                .map_err(|err| {
                    println!("Error. {err}");
                    err
                })?;

            fs.create(&data.name, &data.content, true).await?;
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
