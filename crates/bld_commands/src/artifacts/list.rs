use crate::command::BldCommand;
use actix_web::rt::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_http::HttpClient;
use bld_utils::sync::IntoArc;
use clap::Args;
use tabled::{Table, Tabled, settings::Style};

#[derive(Tabled)]
struct ArtifactInfoRow<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub date_created: &'a str,
    pub date_expires: &'a str,
}

#[derive(Args)]
#[command(about = "Lists the artifacts of a pipeline run on a server")]
pub struct ArtifactsListCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server to list the artifacts from"
    )]
    server: String,

    #[arg(
        short = 'r',
        long = "run-id",
        help = "The id of the pipeline run whose artifacts to list"
    )]
    run_id: String,
}

impl BldCommand for ArtifactsListCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        System::new().block_on(async move {
            let config = BldConfig::load().await?.into_arc();
            let client = HttpClient::new(config, &self.server)?;
            let response = client.artifacts_list(&self.run_id).await?;

            if !response.is_empty() {
                let data: Vec<ArtifactInfoRow> = response
                    .iter()
                    .map(|a| ArtifactInfoRow {
                        id: &a.id,
                        name: &a.name,
                        date_created: &a.date_created,
                        date_expires: &a.date_expires,
                    })
                    .collect();
                let table = Table::new(data).with(Style::modern()).to_string();
                println!("{table}");
            }

            Ok(())
        })
    }
}
