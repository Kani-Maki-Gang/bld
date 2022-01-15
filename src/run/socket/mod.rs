mod client;
pub mod messages;

use crate::run::socket::client::ExecClient;
use crate::run::socket::messages::ExecInfo;
use actix::{io::SinkWrite, Actor, StreamHandler};
use actix_web::rt::System;
use anyhow::anyhow;
use awc::Client;
use futures::stream::StreamExt;
use std::collections::HashMap;
use tracing::debug;

pub struct ExecConnectionInfo {
    pub host: String,
    pub port: i64,
    pub headers: HashMap<String, String>,
    pub detach: bool,
    pub pipeline: String,
    pub variables: HashMap<String, String>,
}

async fn remote_invoke(info: ExecConnectionInfo) -> anyhow::Result<()> {
    let url = format!("http://{}:{}/ws-exec/", info.host, info.port);
    debug!("establishing web socker connection on {}", url);
    let mut client = Client::new().ws(url);
    for (key, value) in info.headers.iter() {
        client = client.header(&key[..], &value[..]);
    }
    let (_, framed) = client.connect().await.map_err(|e| anyhow!(e.to_string()))?;
    let (sink, stream) = framed.split();
    let addr = ExecClient::create(|ctx| {
        ExecClient::add_stream(stream, ctx);
        ExecClient::new(SinkWrite::new(sink, ctx))
    });
    debug!(
        "sending data over: {:?} {:?}",
        info.pipeline, info.variables
    );
    if info.detach {
        addr.do_send(ExecInfo::new(&info.pipeline, Some(info.variables.clone())));
    } else {
        addr.send(ExecInfo::new(&info.pipeline, Some(info.variables.clone())))
            .await?;
    }
    Ok(())
}

pub fn on_server(info: ExecConnectionInfo) -> anyhow::Result<()> {
    debug!("spawing actix system");
    let sys = System::new();
    let res = sys.block_on(remote_invoke(info));
    sys.run()?;
    res
}
