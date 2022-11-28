use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, Scope, TokenUrl};
use serde::{Serialize, Deserialize};

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
    Ldap,
    OAuth2(Box<OAuth2Info>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthValidation {
    Ldap,
    OAuth2(String),
}
