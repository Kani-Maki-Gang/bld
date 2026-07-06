use crate::command::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_http::HttpClient;
use bld_utils::sync::IntoArc;
use clap::Args;

#[derive(Args)]
#[command(about = "Downloads an artifact from a server")]
pub struct ArtifactsDownloadCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(required = true, help = "The id of the artifact to download")]
    id: String,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server to download the artifact from"
    )]
    server: String,

    #[arg(
        short = 'o',
        long = "output",
        help = "The local path to save the downloaded artifact to"
    )]
    output: Option<String>,
}

impl BldCommand for ArtifactsDownloadCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        System::new().block_on(async move {
            let config = BldConfig::load().await?.into_arc();
            let client = HttpClient::new(config, &self.server)?;
            let bytes = client.artifacts_download(&self.id).await?;
            let output = self.output.unwrap_or_else(|| format!("{}.tar.gz", self.id));
            tokio::fs::write(&output, bytes).await?;
            println!("Downloaded artifact {} to {output}", self.id);
            Ok(())
        })
    }
}
