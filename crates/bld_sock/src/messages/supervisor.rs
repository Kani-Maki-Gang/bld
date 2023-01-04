use actix::Message;
use serde::{Deserialize, Serialize};

pub static SERVER: &str = "server";
pub static WORKER: &str = "worker";

#[derive(Debug, Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub enum ServerMessages {
    Ack,
    Enqueue {
        pipeline: String,
        run_id: String,
        variables: Option<Vec<String>>,
        environment: Option<Vec<String>>,
    },
    Stop {
        run_id: String,
    },
}

#[derive(Debug, Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub enum WorkerMessages {
    Ack,
    WhoAmI { pid: u32 },
    Completed,
}
