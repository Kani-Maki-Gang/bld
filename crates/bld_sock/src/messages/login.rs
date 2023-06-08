use actix::Message;
use bld_core::auth::AuthTokens;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub enum LoginClientMessage {
    Init,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LoginServerMessage {
    AuthorizationUrl(String),
    Completed(AuthTokens),
    Failed { reason: String },
}
