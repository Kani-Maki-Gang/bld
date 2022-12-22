use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, Scope, TokenUrl};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Info {
    pub auth_url: AuthUrl,
    pub token_url: TokenUrl,
    pub redirect_url: RedirectUrl,
    pub client_id: ClientId,
    pub client_secret: ClientSecret,
    pub scopes: Vec<Scope>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum Auth {
    #[serde(rename(serialize = "ldap", deserialize = "ldap"))]
    Ldap,

    #[serde(rename(serialize = "oauth2", deserialize = "oauth2"))]
    OAuth2(Box<OAuth2Info>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum AuthValidation {
    #[serde(rename(serialize = "ldap", deserialize = "ldap"))]
    Ldap,

    #[serde(rename(serialize = "oauth2", deserialize = "oauth2"))]
    OAuth2 { validation_url: String },
}
