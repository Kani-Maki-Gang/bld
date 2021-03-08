use crate::config::definitions::TOOL_DEFAULT_PIPELINE;
use crate::persist::{NullExec, ShellLogger};
use crate::run::{self, Runner};
use crate::types::Result;
use clap::ArgMatches;
use tokio::runtime::Runtime;
use std::collections::HashMap;
use std::sync::Arc;

pub fn exec(matches: &ArgMatches<'_>) -> Result<()> {
    let pipeline = matches
        .value_of("pipeline")
        .or(Some(TOOL_DEFAULT_PIPELINE))
        .unwrap()
        .to_string();
    let detach = matches.is_present("detach");
    let vars = Arc::new(matches
        .values_of("variables")
        .map(|variable| {
            variable
                .map(|v| {
                    let mut split = v.split('=');
                    let name = split.next().or(Some("")).unwrap().to_string();
                    let value = split.next().or(Some("")).unwrap().to_string();
                    (name, value)
                })
                .collect::<HashMap<String, String>>()
        })
        .or_else(|| Some(HashMap::new()))
        .unwrap());
    match matches.value_of("server") {
        Some(server) => run::on_server(pipeline, server.to_string(), detach),
        None => {
            let mut rt = Runtime::new()?;
            rt.block_on(async {
                Runner::from_file(pipeline, NullExec::atom(), ShellLogger::atom(), None, vars)
                    .await
                    .await
            })
        }
    }
}
