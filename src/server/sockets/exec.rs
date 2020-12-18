use crate::config::BldConfig;
use crate::path;
use crate::persist::{Database, FileLogger, FileScanner, Scanner};
use crate::run::{Pipeline, Runner};
use crate::term;
use crate::types::{BldError, Result};
use actix::prelude::*;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use uuid::Uuid;

type StdResult<T, V> = std::result::Result<T, V>;
type AtomicDb = Arc<Mutex<Database>>;
type AtomicFs = Arc<Mutex<FileLogger>>;
type AtomicRecv = Arc<Mutex<Receiver<bool>>>;

pub struct ExecutePipelineSocket {
    hb: Instant,
    config: Option<BldConfig>,
    exec: Option<Arc<Mutex<Database>>>,
    logger: Option<Arc<Mutex<FileLogger>>>,
    scanner: Option<FileScanner>,
}

impl ExecutePipelineSocket {
    pub fn new() -> Self {
        let config = match BldConfig::load() {
            Ok(config) => Some(config),
            Err(_) => None,
        };
        Self {
            hb: Instant::now(),
            config,
            exec: None,
            logger: None,
            scanner: None,
        }
    }

    fn heartbeat(act: &Self, ctx: &mut <Self as Actor>::Context) {
        if Instant::now().duration_since(act.hb) > Duration::from_secs(10) {
            println!("Websocket heartbeat failed, disconnecting!");
            ctx.stop();
            return;
        }
        ctx.ping(b"");
    }

    fn scan(act: &mut Self, ctx: &mut <Self as Actor>::Context) {
        if let Some(scanner) = act.scanner.as_mut() {
            let content = scanner.fetch();
            for line in content.iter() {
                ctx.text(line);
            }
        }
    }

    fn exec(act: &mut Self, ctx: &mut <Self as Actor>::Context) {
        if let Some(exec) = act.exec.as_mut() {
            let exec = exec.lock().unwrap();
            if let Some(pipeline) = &exec.pipeline {
                if !pipeline.running {
                    ctx.stop();
                }
            }
        }
    }

    fn dependencies(&mut self, name: &str) -> Result<()> {
        let path = Pipeline::get_path(name)?;
        if !path.is_file() {
            let message = String::from("pipeline file not found");
            return Err(BldError::IoError(message));
        }

        let id = Uuid::new_v4().to_string();
        let config = match &self.config {
            Some(config) => config,
            None => return Err(BldError::Other("config not loaded".to_string())),
        };
        let lg_path = path![&config.local.logs, format!("{}-{}", name, id)]
            .display()
            .to_string();
        let mut db = Database::connect(&config.local.db)?;
        let _ = db.add(&id, &name)?;
        let lg = FileLogger::new(&lg_path)?;

        self.exec = Some(Arc::new(Mutex::new(db)));
        self.logger = Some(Arc::new(Mutex::new(lg)));
        self.scanner = Some(FileScanner::new(&lg_path)?);

        Ok(())
    }
}

impl Actor for ExecutePipelineSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(Duration::from_secs(1), |act, ctx| {
            ExecutePipelineSocket::heartbeat(act, ctx);
            ExecutePipelineSocket::scan(act, ctx);
        });
        ctx.run_interval(Duration::from_secs(5), |act, ctx| {
            ExecutePipelineSocket::exec(act, ctx);
        });
    }
}

impl StreamHandler<StdResult<ws::Message, ws::ProtocolError>> for ExecutePipelineSocket {
    fn handle(&mut self, msg: StdResult<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(txt)) => {
                if let Err(e) = self.dependencies(&txt) {
                    eprintln!("{}", e.to_string());
                    ctx.text("Unable to run pipeline");
                    ctx.stop();
                }
                let ex = match &self.exec {
                    Some(exec) => exec.clone(),
                    None => {
                        ctx.stop();
                        return;
                    }
                };
                let lg = match &self.logger {
                    Some(lg) => lg.clone(),
                    None => {
                        ctx.stop();
                        return;
                    }
                };
                std::thread::spawn(move || invoke_pipeline(txt, ex, lg, None));
            }
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
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

fn invoke_pipeline(name: String, ex: AtomicDb, lg: AtomicFs, cm: Option<AtomicRecv>) {
    if let Ok(mut rt) = Runtime::new() {
        rt.block_on(async move {
            let fut = Runner::from_file(name, ex, lg, cm);
            if let Err(e) = fut.await.await {
                let _ = term::print_error(&e.to_string());
            }
        });
    }
}

pub async fn ws_exec(req: HttpRequest, stream: web::Payload) -> StdResult<HttpResponse, Error> {
    println!("{:?}", req);
    let res = ws::start(ExecutePipelineSocket::new(), &req, stream);
    println!("{:?}", res);
    res
}
