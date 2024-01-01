use actix::Message;
use serde::{Deserialize, Serialize};

use crate::auth::AuthTokens;

#[derive(Debug, Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub enum LoginClientMessage {
    Init,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LoginServerMessage {
    AuthorizationUrl(String),
    Completed(AuthTokens),
    Failed(String),
}
