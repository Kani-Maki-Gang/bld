#[cfg(feature = "web_socket")]
use actix::Message;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

impl AuthTokens {
    pub fn new(access_token: String, refresh_token: Option<String>) -> Self {
        Self {
            access_token,
            refresh_token,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenParams {
    pub refresh_token: String,
}

impl RefreshTokenParams {
    pub fn new(refresh_token: &str) -> Self {
        Self {
            refresh_token: refresh_token.to_owned(),
        }
    }
}

#[cfg(feature = "web_socket")]
#[derive(Debug, Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub enum LoginClientMessage {
    Init,
}

#[cfg(feature = "web_socket")]
#[derive(Debug, Serialize, Deserialize)]
pub enum LoginServerMessage {
    AuthorizationUrl(String),
    Completed(AuthTokens),
    Failed(String),
}
