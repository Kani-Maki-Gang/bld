use std::collections::HashMap;

#[cfg(feature = "web_socket")]
use actix::Message;
use serde::{Deserialize, Serialize};

#[cfg(feature = "web_socket")]
#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub enum ExecClientMessage {
    EnqueueRun {
        name: String,
        environment: Option<HashMap<String, String>>,
        variables: Option<HashMap<String, String>>,
    },
}

#[cfg(feature = "web_socket")]
#[derive(Debug, Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub enum ExecServerMessage {
    QueuedRun { run_id: String },
    Log { content: String },
}
