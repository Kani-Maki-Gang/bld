use crate::types::{BldError, Result};
use openidconnect::{ClientId, ClientSecret, IssuerUrl};
use yaml_rust::Yaml;

#[derive(Debug)]
pub struct OpenIdInfo {
    pub issuer_url: IssuerUrl,
    pub client_id: ClientId,
    pub client_secret: ClientSecret,
    pub scopes: Vec<String>,
}

impl OpenIdInfo {
    pub fn load(yaml: &Yaml) -> Result<Self> {
        let issuer_url = match yaml["issuer"].as_str() {
            Some(url) => IssuerUrl::new(url.to_string())?,
            None => {
                let message = "No issuer url found in config".to_string();
                return Err(BldError::Other(message));
            }
        };
        let client_id = match yaml["client-id"].as_str() {
            Some(id) => ClientId::new(id.to_string()),
            None => {
                let message = "No client id found in config".to_string();
                return Err(BldError::Other(message));
            }
        };
        let client_secret = match yaml["client-secret"].as_str() {
            Some(secret) => ClientSecret::new(secret.to_string()),
            None => {
                let message = "No client secret found in config".to_string();
                return Err(BldError::Other(message));
            }
        };
        let scopes = match yaml["scopes"].as_vec() {
            Some(scopes) => scopes
                .iter()
                .map(|y| y.as_str())
                .filter(|y| y.is_some())
                .map(|y| y.unwrap().to_string())
                .collect(),
            None => Vec::<String>::new(),
        };
        Ok(Self { issuer_url, client_id, client_secret, scopes })
    }
}

#[derive(Debug)]
pub enum Auth {
    Ldap,
    OpenId(OpenIdInfo),
    None
}

#[derive(Debug)]
pub struct BldServerConfig {
    pub name: String,
    pub host: String,
    pub port: i64,
    pub auth: Auth,
}

impl BldServerConfig {
    pub fn load(yaml: &Yaml) -> Result<Self> {
        let name = match yaml["server"].as_str() {
            Some(name) => name.to_string(),
            None => {
                let message = "Server entry must have a name".to_string();
                return Err(BldError::Other(message));
            }
        };
        let host = match yaml["host"].as_str() {
            Some(host) => host.to_string(),
            None => {
                let message = "Server entry must have a host address".to_string();
                return Err(BldError::Other(message));
            }
        };
        let port = match yaml["port"].as_i64() {
            Some(port) => port,
            None => {
                let message = "Server entry must define a port".to_string();
                return Err(BldError::Other(message));
            }
        };
        let auth = match yaml["auth"]["method"].as_str() {
            Some("ldap") => Auth::Ldap,
            Some("openid") => Auth::OpenId(OpenIdInfo::load(&yaml["auth"])?),
            _ => Auth::None,
        };
        Ok(Self { name, host, port, auth })
    }
}
