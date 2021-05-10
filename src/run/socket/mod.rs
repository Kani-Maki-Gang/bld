mod client;

pub use client::*;

use crate::config::BldConfig;
use crate::helpers::errors::auth_for_server_invalid;
use crate::helpers::request::headers;
use crate::helpers::term::print_error;
use crate::run::socket::ExecClient;
use crate::types::{ExecInfo, Result};
use actix::{io::SinkWrite, Actor, Arbiter, StreamHandler, System};
use awc::Client;
use futures::stream::StreamExt;
use std::collections::HashMap;

async fn remote_invoke(server: String, detach: bool, data: ExecInfo) -> Result<bool> {
    let config = BldConfig::load()?;
    let srv = config.remote.server(&server)?;
    let (srv_name, auth) = match &srv.same_auth_as {
        Some(name) => match config.remote.servers.iter().find(|s| &s.name == name) {
            Some(srv) => (&srv.name, &srv.auth),
            None => return auth_for_server_invalid().map(|_| false),
        },
        None => (&srv.name, &srv.auth),
    };
    let url = format!("http://{}:{}/ws-exec/", srv.host, srv.port);
    let headers = headers(srv_name, auth)?;
    let mut client = Client::new().ws(url);
    for (key, value) in headers.iter() {
        client = client.header(&key[..], &value[..]);
    }
    let (_, framed) = client.connect().await?;
    println!("connected");
    let (sink, stream) = framed.split();
    let addr = ExecClient::create(|ctx| {
        ExecClient::add_stream(stream, ctx);
        ExecClient::new(SinkWrite::new(sink, ctx))
    });
    if detach {
        addr.do_send(data);
        Ok(true)
    } else {
        let _ = addr.send(data).await;
        Ok(false)
    }
}

pub fn on_server(
    name: String,
    vars: HashMap<String, String>,
    server: String,
    detach: bool,
) -> Result<()> {
    let system = System::new("bld");
    let data = ExecInfo::new(&name, Some(vars));
    Arbiter::spawn(async move {
        match remote_invoke(server, detach, data).await {
            Ok(true) => System::current().stop(),
            Err(e) => {
                let _ = print_error(&e.to_string());
                System::current().stop();
            }
            _ => {}
        }
    });
    let _ = system.run();
    Ok(())
}
