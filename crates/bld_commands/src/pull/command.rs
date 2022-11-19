use crate::command::BldCommand;
use actix_web::rt::System;
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_server::responses::PullResponse;
use bld_utils::fs::IsYaml;
use bld_utils::request::Request;
use clap::Args;
use std::fs::{create_dir_all, remove_file, File};
use std::io::Write;
use tracing::debug;

#[derive(Args)]
#[command(about = "Pull a pipeline from a bld server and stores it localy")]
pub struct PullCommand {
    #[arg(short = 'p', long = "pipeline", required = true, help = "The name of the bld server")]
    pipeline: String,

    #[arg(short = 's', long = "server", help = "The name of the bld server")]
    server: Option<String>,

    #[arg(long = "ignore-deps", help = "Do not include other pipeline dependencies")]
    ignore_deps: bool,
}

impl PullCommand {
    async fn request(self) -> Result<()> {
        let config = BldConfig::load()?;
        let server = config
            .remote
            .server_or_first(self.server.as_ref())?;
        let server_auth = config.remote.same_auth_as(server)?;
        let protocol = server.http_protocol();
        let metadata_url = format!("{protocol}://{}:{}/deps", server.host, server.port);
        let url = format!("{protocol}://{}:{}/pull", server.host, server.port);

        debug!(
            "running pull subcommand with --server: {}, --pipeline: {} and --ignore-deps: {}",
            server.name, self.pipeline, self.ignore_deps
        );

        let mut pipelines = vec![self.pipeline.to_string()];

        if !self.ignore_deps {
            debug!("sending http request to {}", metadata_url);
            print!("Fetching metadata for dependecies...");

            Request::post(&metadata_url)
                .auth(&server_auth)
                .send_json(self.pipeline)
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

            Request::post(&url)
                .auth(&server_auth)
                .send_json(pipeline.to_string())
                .await
                .and_then(Self::save)
                .map(|_| {
                    println!("Done.");
                })
                .map_err(|e| {
                    println!("Error. {e}");
                    e
                })?;
        }

        Ok(())
    }

    fn save(data: PullResponse) -> Result<()> {
        let path = PipelineFileSystemProxy::Local.path(&data.name)?;

        if path.is_yaml() {
            remove_file(&path)?;
        } else if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }

        let mut handle = File::create(&path)?;
        handle.write_all(data.content.as_bytes())?;

        Ok(())
    }
}

impl BldCommand for PullCommand {
    fn exec(self) -> Result<()> {
        System::new().block_on(self.request())
    }
}
