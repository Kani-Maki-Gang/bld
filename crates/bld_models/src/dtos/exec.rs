use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "web_socket")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecClientMessage {
    EnqueueRun {
        name: String,
        env: Option<HashMap<String, String>>,
        inputs: Option<HashMap<String, String>>,
    },
}

#[cfg(feature = "web_socket")]
#[derive(Debug, Serialize, Deserialize)]
pub enum ExecServerMessage {
    QueuedRun { run_id: String },
    Log { content: String },
}
