use std::collections::HashMap;

use actix::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub enum ExecClientMessage {
    EnqueueRun {
        name: String,
        environment: Option<HashMap<String, String>>,
        variables: Option<HashMap<String, String>>,
    },
}

#[derive(Debug, Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub enum ExecServerMessage {
    QueuedRun { run_id: String },
    Log { content: String },
}
