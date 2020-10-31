use crate::config::BldConfig;
use crate::persist::FileSystemDumpster;
use crate::run::Runner;
use crate::term;
use actix::prelude::*;
use actix_web_actors::ws;
use notify::{watcher, DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use serde_json::Value;
use std::io::{self, Error, ErrorKind};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::runtime::Runtime;

pub struct PipelineWebSocketServer {
    hb: Instant,
    watcher: Option<RecommendedWatcher>,
    file_rx: Option<Receiver<DebouncedEvent>>,
    is_pipeline_done: bool,
}

impl PipelineWebSocketServer {
    pub fn new() -> Self {
        Self {
            hb: Instant::now(),
            watcher: None,
            file_rx: None,
            is_pipeline_done: false,
        }
    }

    fn heartbeat(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(Duration::from_secs(5), |act, ctx| {
            if Instant::now().duration_since(act.hb) > Duration::from_secs(10) {
                println!("Websocket heartbeat failed, disconnecting!");
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }
}

impl Actor for PipelineWebSocketServer {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.heartbeat(ctx);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for PipelineWebSocketServer {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        if self.is_pipeline_done {
            ctx.close(None);
        }
        match msg {
            Ok(ws::Message::Text(txt)) => {
                if let Some((path, src)) = parse_text(&txt) {
                    let (tx, rx) = mpsc::channel::<DebouncedEvent>();
                    if let Ok((watcher, dumpster)) = create_wd(tx, &path) {
                        self.watcher = Some(watcher);
                        self.file_rx = Some(rx);
                        std::thread::spawn(move || invoke_pipeline(src, dumpster));
                    }
                }
            }
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                if let Some(rx) = &self.file_rx {
                    match rx.recv() {
                        Ok(_) => {
                            ctx.text("cool new update yall");
                        }
                        Err(e) => {
                            eprintln!("{:?}", e);
                            // ctx.text("could not update info");
                            // ctx.stop();
                        }
                    }
                }
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

fn parse_text(text: &str) -> Option<(String, String)> {
    if let Ok(config) = BldConfig::load() {
        if let Ok(message) = serde_json::from_str::<Value>(text) {
            let name = match message["name"].as_str() {
                Some(name) => {
                    let time = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("Time went backwards");
                    format!("{}-{}", name, time.as_nanos())
                }
                None => return None,
            };
            let path = {
                let mut path = std::path::PathBuf::new();
                path.push(config.local.logs);
                path.push(name);
                path.display().to_string()
            };
            let src = match message["pipeline"].as_str() {
                Some(src) => src.to_string(),
                None => return None,
            };
            return Some((path, src));
        }
    }
    None
}

fn create_wd(
    tx: Sender<DebouncedEvent>,
    path: &str,
) -> io::Result<(RecommendedWatcher, FileSystemDumpster)> {
    let dumpster = match FileSystemDumpster::new(&path) {
        Ok(dumpster) => dumpster,
        Err(_) => return Err(Error::new(ErrorKind::Other, "could not create fs dumpster")),
    };
    let mut watcher = match watcher(tx, Duration::from_secs(2)) {
        Ok(watcher) => watcher,
        Err(_) => return Err(Error::new(ErrorKind::Other, "could not create watcher")),
    };
    if let Err(_) = watcher.watch(path, RecursiveMode::Recursive) {
        return Err(Error::new(
            ErrorKind::Other,
            "could not watch the provided path",
        ));
    }
    Ok((watcher, dumpster))
}

fn invoke_pipeline(src: String, dumpster: FileSystemDumpster) {
    if let Ok(mut rt) = Runtime::new() {
        rt.block_on(async move {
            let dumpster = Arc::new(Mutex::new(dumpster));
            if let Err(e) = Runner::from_src(src, dumpster).await.await {
                let _ = term::print_error(&format!("{}", e));
            }
        });
    }
}
