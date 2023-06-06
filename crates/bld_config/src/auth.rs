use std::fmt::{Display, Formatter, Result as FmtResult};

use anyhow::Result;
use openidconnect::{
    core::{CoreClient, CoreProviderMetadata},
    reqwest::async_http_client,
    AuthUrl, ClientId, ClientSecret, IssuerUrl, RedirectUrl, Scope, TokenUrl,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserInfoProperty {
    #[serde(rename = "name")]
    Name,
    #[serde(rename = "email")]
    Email,
}

impl Display for UserInfoProperty {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Name => write!(f, "name"),
            Self::Email => write!(f, "email"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenIdInfo {
    pub issuer_url: IssuerUrl,
    pub redirect_url: RedirectUrl,
    pub client_id: ClientId,
    pub client_secret: ClientSecret,
    pub scopes: Vec<Scope>,
    pub user_property: UserInfoProperty,
}

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
    #[serde(rename(serialize = "oidc", deserialize = "oidc"))]
    OpenId(Box<OpenIdInfo>),
}

impl Auth {
    pub async fn core_client(&self) -> Result<CoreClient> {
        let Self::OpenId(openid) = self;

        let provider_metadata =
            CoreProviderMetadata::discover_async(openid.issuer_url.clone(), async_http_client)
                .await?;

        let client = CoreClient::from_provider_metadata(
            provider_metadata,
            openid.client_id.clone(),
            Some(openid.client_secret.clone()),
        )
        .set_redirect_uri(openid.redirect_url.clone());

        Ok(client)
    }
}
