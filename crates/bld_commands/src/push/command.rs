use crate::command::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_runner::VersionedPipeline;
use bld_server::requests::PushInfo;
use bld_utils::request::Request;
use clap::Args;
use tracing::debug;

#[derive(Args)]
#[command(about = "Pushes the contents of a pipeline to a bld server")]
pub struct PushCommand {
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
    server: Option<String>,

    #[arg(
        long = "ignore-deps",
        help = "Don't include other pipeline dependencies"
    )]
    ignore_deps: bool,
}

impl PushCommand {
    async fn push(self) -> Result<()> {
        let config = BldConfig::load()?;
        let server = config.server_or_first(self.server.as_ref())?;

        debug!(
            "running push subcommand with --server: {} and --pipeline: {}",
            server.name, self.pipeline
        );

        let server_auth = config.same_auth_as(server)?;
        let url = format!(
            "{}://{}:{}/push",
            server.http_protocol(),
            server.host,
            server.port
        );

        let mut pipelines = vec![PushInfo::new(
            &self.pipeline,
            &PipelineFileSystemProxy::Local.read(&self.pipeline)?,
        )];

        if !self.ignore_deps {
            print!("Resolving dependecies...");

            let mut deps =
                VersionedPipeline::dependencies(&PipelineFileSystemProxy::Local, &self.pipeline)
                    .map(|pips| {
                        println!("Done.");
                        pips.iter().map(|(n, s)| PushInfo::new(n, s)).collect()
                    })
                    .map_err(|e| {
                        println!("Error. {e}");
                        e
                    })?;

            pipelines.append(&mut deps);
        }

        for info in pipelines.into_iter() {
            print!("Pushing {}...", info.name);

            debug!("sending request to {url}");

            let _ = Request::post(&url)
                .auth(server_auth)
                .send_json(info)
                .await
                .map(|_: String| {
                    println!("Done.");
                })
                .map_err(|e| {
                    println!("Error. {e}");
                    e
                });
        }
        Ok(())
    }
}

impl BldCommand for PushCommand {
    fn exec(self) -> Result<()> {
        System::new().block_on(self.push())
    }
}
