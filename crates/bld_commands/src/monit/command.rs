use crate::command::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::logger::Logger;
use bld_models::dtos::MonitInfo;
use bld_sock::MonitClient;
use clap::Args;
use tracing::debug;

#[derive(Args)]
#[command(about = "Connects to a bld server to monitor the execution of a file")]
pub struct MonitCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(
        short = 'i',
        long = "pipeline-id",
        help = "The id of the run to monitor. Takes precedence over the file name"
    )]
    pipeline_id: Option<String>,

    #[arg(
        short = 'f',
        long = "file",
        help = "The name of the file of which to monitor the last run"
    )]
    file: Option<String>,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server to monitor the file from"
    )]
    server: String,

    #[arg(
        long = "last",
        help = "Monitor the execution of the last invoked file. Takes precedence over pipeline-id and file"
    )]
    last: bool,
}

impl MonitCommand {
    async fn request(self) -> Result<()> {
        let config = BldConfig::load().await?;
        let logger = Logger::shell();
        debug!(
            "sending data over: {:?} {:?} {}",
            self.pipeline_id, self.file, self.last
        );
        MonitClient::connect(config, logger, self.server)
            .await?
            .run(MonitInfo::new(self.pipeline_id, self.file, self.last))
            .await
    }
}

impl BldCommand for MonitCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        System::new().block_on(self.request())
    }
}
