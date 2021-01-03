use actix::MailboxError;
use actix_http::http::uri::InvalidUri;
use actix_web::client::WsClientError;
use diesel::ConnectionError;
use openidconnect::url::ParseError;
use openidconnect::DiscoveryError;
use std::convert::From;
use std::error;
use std::io;
use std::marker::{Send, Sync};
use yaml_rust::scanner::ScanError;

pub type Result<T> = std::result::Result<T, BldError>;

pub enum BldError {
    ActixError(String),
    DieselError(String),
    IoError(String),
    SerdeError(String),
    ShipliftError(String),
    YamlError(String),
    OpenIdConnectError(String),
    Other(String),
}

impl From<io::Error> for BldError {
    fn from(error: io::Error) -> Self {
        BldError::IoError(error.to_string())
    }
}

impl From<ScanError> for BldError {
    fn from(error: ScanError) -> Self {
        BldError::YamlError(error.to_string())
    }
}

impl From<InvalidUri> for BldError {
    fn from(error: InvalidUri) -> Self {
        BldError::ActixError(error.to_string())
    }
}

impl From<MailboxError> for BldError {
    fn from(error: MailboxError) -> Self {
        BldError::ActixError(error.to_string())
    }
}

impl From<WsClientError> for BldError {
    fn from(error: WsClientError) -> Self {
        BldError::ActixError(error.to_string())
    }
}

impl From<diesel::result::Error> for BldError {
    fn from(error: diesel::result::Error) -> Self {
        BldError::DieselError(error.to_string())
    }
}

impl From<ConnectionError> for BldError {
    fn from(error: ConnectionError) -> Self {
        BldError::DieselError(error.to_string())
    }
}

impl From<serde_json::Error> for BldError {
    fn from(error: serde_json::Error) -> Self {
        BldError::SerdeError(error.to_string())
    }
}

impl From<shiplift::Error> for BldError {
    fn from(error: shiplift::Error) -> Self {
        BldError::ShipliftError(error.to_string())
    }
}

impl From<ParseError> for BldError {
    fn from(error: ParseError) -> Self {
        BldError::OpenIdConnectError(error.to_string())
    }
}

impl<T: 'static + Sync + Send + error::Error> From<DiscoveryError<openidconnect::reqwest::Error<T>>> for BldError {
    fn from(error: DiscoveryError<openidconnect::reqwest::Error<T>>) -> Self {
        BldError::OpenIdConnectError(error.to_string())
    }
}

impl std::string::ToString for BldError {
    fn to_string(&self) -> String {
        match self {
            BldError::ActixError(a) => a.to_string(),
            BldError::DieselError(d) => d.to_string(),
            BldError::IoError(i) => i.to_string(),
            BldError::SerdeError(s) => s.to_string(),
            BldError::ShipliftError(s) => s.to_string(),
            BldError::YamlError(y) => y.to_string(),
            BldError::OpenIdConnectError(o) => o.to_string(),
            BldError::Other(o) => o.to_string(),
        }
    }
}
