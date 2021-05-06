use actix::MailboxError;
use actix_http::client::SendRequestError;
use actix_http::error::PayloadError;
use actix_http::http::uri::InvalidUri;
use actix_web::client::WsClientError;
use diesel::ConnectionError;
use oauth2::basic::BasicErrorResponseType;
use oauth2::reqwest::Error as ReqError;
use oauth2::url::ParseError;
use oauth2::{RequestTokenError, StandardErrorResponse};
use std::convert::From;
use std::error::Error;
use std::io;
use std::marker::{Send, Sync};
use std::str::ParseBoolError;
use yaml_rust::scanner::ScanError;

pub type Result<T> = std::result::Result<T, BldError>;

pub enum BldError {
    ActixError(String),
    DieselError(String),
    IoError(String),
    ParseError(String),
    SerdeError(String),
    ShipliftError(String),
    YamlError(String),
    OAuth2(String),
    Other(String),
}

impl From<io::Error> for BldError {
    fn from(error: io::Error) -> Self {
        Self::IoError(error.to_string())
    }
}

impl From<ScanError> for BldError {
    fn from(error: ScanError) -> Self {
        Self::YamlError(error.to_string())
    }
}

impl From<InvalidUri> for BldError {
    fn from(error: InvalidUri) -> Self {
        Self::ActixError(error.to_string())
    }
}

impl From<MailboxError> for BldError {
    fn from(error: MailboxError) -> Self {
        Self::ActixError(error.to_string())
    }
}

impl From<WsClientError> for BldError {
    fn from(error: WsClientError) -> Self {
        Self::ActixError(error.to_string())
    }
}

impl From<SendRequestError> for BldError {
    fn from(error: SendRequestError) -> Self {
        Self::ActixError(error.to_string())
    }
}

impl From<&mut SendRequestError> for BldError {
    fn from(error: &mut SendRequestError) -> Self {
        Self::ActixError(error.to_string())
    }
}

impl From<PayloadError> for BldError {
    fn from(error: PayloadError) -> Self {
        Self::ActixError(error.to_string())
    }
}

impl From<diesel::result::Error> for BldError {
    fn from(error: diesel::result::Error) -> Self {
        Self::DieselError(error.to_string())
    }
}

impl From<ConnectionError> for BldError {
    fn from(error: ConnectionError) -> Self {
        Self::DieselError(error.to_string())
    }
}

impl From<serde_json::Error> for BldError {
    fn from(error: serde_json::Error) -> Self {
        Self::SerdeError(error.to_string())
    }
}

impl From<shiplift::Error> for BldError {
    fn from(error: shiplift::Error) -> Self {
        Self::ShipliftError(error.to_string())
    }
}

impl From<&str> for BldError {
    fn from(error: &str) -> Self {
        Self::Other(error.to_string())
    }
}

impl From<ParseError> for BldError {
    fn from(error: ParseError) -> Self {
        Self::OAuth2(error.to_string())
    }
}

impl<T: 'static + Sync + Send + Error>
    From<RequestTokenError<ReqError<T>, StandardErrorResponse<BasicErrorResponseType>>>
    for BldError
{
    fn from(
        error: RequestTokenError<ReqError<T>, StandardErrorResponse<BasicErrorResponseType>>,
    ) -> Self {
        Self::OAuth2(error.to_string())
    }
}

impl From<ParseBoolError> for BldError {
    fn from(error: ParseBoolError) -> Self {
        Self::ParseError(error.to_string())
    }
}

impl std::string::ToString for BldError {
    fn to_string(&self) -> String {
        match self {
            Self::ActixError(a) => a.to_string(),
            Self::DieselError(d) => d.to_string(),
            Self::IoError(i) => i.to_string(),
            Self::ParseError(p) => p.to_string(),
            Self::SerdeError(s) => s.to_string(),
            Self::ShipliftError(s) => s.to_string(),
            Self::YamlError(y) => y.to_string(),
            Self::OAuth2(o) => o.to_string(),
            Self::Other(o) => o.to_string(),
        }
    }
}
