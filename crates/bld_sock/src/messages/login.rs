use actix::Message;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub enum LoginClientMessage {
    Init,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LoginServerMessage {
    AuthorizationUrl(String),
    Completed { access_token: String },
    Failed { reason: String },
}
