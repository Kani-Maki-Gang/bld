use crate::config::{Auth, BldServerConfig};
use crate::types::{BldError, Result};
use openidconnect::core::{CoreClient, CoreProviderMetadata, CoreResponseType};
use openidconnect::reqwest::http_client;
use openidconnect::{AuthenticationFlow, CsrfToken, Scope, Nonce, RedirectUrl};

pub trait Login {
    fn login(&self) -> Result<String>;
}

impl Login for BldServerConfig {
    fn login(&self) -> Result<String> {
        match &self.auth {
            Auth::OpenId(_) => openid_login(self),
            Auth::Ldap => ldap_login(self),
            _ => unreachable!(),
        }
    }
}

fn openid_login(config: &BldServerConfig) -> Result<String> {
    let info = match &config.auth {
        Auth::OpenId(info) => info,
        _ => return Err(BldError::Other("invalid auth method".to_string())),
    };
    let provider = CoreProviderMetadata::discover(&info.issuer_url, http_client)?;
    let client = CoreClient::from_provider_metadata(provider, info.client_id.clone(), Some(info.client_secret.clone()))
        .set_redirect_uri(RedirectUrl::new(format!("http://{}:{}/authRedirect", config.host, config.port))?);
    let mut auth_url = client.authorize_url(
        AuthenticationFlow::<CoreResponseType>::AuthorizationCode, 
        CsrfToken::new_random,
        Nonce::new_random
    );
    for scope in &info.scopes {
        auth_url = auth_url.add_scope(Scope::new(scope.clone()));
    }
    let (auth_url, _, _) = auth_url.url();
    println!("{}", auth_url);
    Ok(String::new())
}

fn ldap_login(_config: &BldServerConfig) -> Result<String> {
    Ok(String::new())
}
