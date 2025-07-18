use std::fmt::{Display, Formatter, Result as FmtResult};

use anyhow::Result;
use openidconnect::{
    AuthUrl, ClientId, ClientSecret, IssuerUrl, RedirectUrl, Scope, TokenUrl,
    core::{CoreClient, CoreProviderMetadata},
    reqwest::async_http_client,
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
    pub client_id: ClientId,
    pub client_secret: ClientSecret,
    pub scopes: Vec<Scope>,
    pub user_property: UserInfoProperty,
}

impl OpenIdInfo {
    async fn build_core_client(&self, redirect_url: RedirectUrl) -> Result<CoreClient> {
        let provider_metadata =
            CoreProviderMetadata::discover_async(self.issuer_url.clone(), async_http_client)
                .await?;

        let client = CoreClient::from_provider_metadata(
            provider_metadata,
            self.client_id.clone(),
            Some(self.client_secret.clone()),
        )
        .set_redirect_uri(redirect_url);

        Ok(client)
    }

    pub async fn core_client(&self, origin: &str) -> Result<CoreClient> {
        let redirect_url = RedirectUrl::new(format!("{origin}/v1/auth/redirect"))?;
        self.build_core_client(redirect_url).await
    }

    pub async fn web_core_client(&self, origin: &str) -> Result<CoreClient> {
        let redirect_url = RedirectUrl::new(format!("{origin}/validate"))?;
        self.build_core_client(redirect_url).await
    }
}

pub struct OAuth2Info {
    pub auth_url: AuthUrl,
    pub token_url: TokenUrl,
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
    pub async fn core_client(&self, origin: &str) -> Result<CoreClient> {
        let Auth::OpenId(open_id) = self;
        open_id.core_client(origin).await
    }

    pub async fn web_core_client(&self, origin: &str) -> Result<CoreClient> {
        let Auth::OpenId(open_id) = self;
        open_id.web_core_client(origin).await
    }
}
