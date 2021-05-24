use crate::types::{Result, EMPTY_YAML_VEC};
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, Scope, TokenUrl};
use yaml_rust::Yaml;

#[derive(Debug)]
pub struct OAuth2Info {
    pub auth_url: AuthUrl,
    pub token_url: TokenUrl,
    pub redirect_url: RedirectUrl,
    pub client_id: ClientId,
    pub client_secret: ClientSecret,
    pub scopes: Vec<Scope>,
}

impl OAuth2Info {
    pub fn load(host: &str, port: i64, yaml: &Yaml) -> Result<Box<Self>> {
        let auth_url = AuthUrl::new(
            yaml["auth-url"]
                .as_str()
                .ok_or("No auth url found in config")?
                .to_string(),
        )?;
        let token_url = TokenUrl::new(
            yaml["token-url"]
                .as_str()
                .ok_or("No token url found in config")?
                .to_string(),
        )?;
        let client_id = ClientId::new(
            yaml["client-id"]
                .as_str()
                .ok_or("No client id found in config")?
                .to_string(),
        );
        let client_secret = ClientSecret::new(
            yaml["client-secret"]
                .as_str()
                .ok_or("No client secret found in config")?
                .to_string(),
        );
        let scopes = yaml["scopes"]
            .as_vec()
            .or(Some(&EMPTY_YAML_VEC))
            .unwrap()
            .iter()
            .map(|y| y.as_str())
            .filter(|y| y.is_some())
            .map(|y| Scope::new(y.unwrap().to_string()))
            .collect();
        let redirect_url = RedirectUrl::new(format!("http://{}:{}/authRedirect", host, port))?;
        Ok(Box::new(Self {
            auth_url,
            token_url,
            redirect_url,
            client_id,
            client_secret,
            scopes,
        }))
    }
}

#[derive(Debug)]
pub enum Auth {
    Ldap,
    OAuth2(Box<OAuth2Info>),
    None,
}

#[derive(Debug, Clone)]
pub enum AuthValidation {
    Ldap,
    OAuth2(String),
    None,
}
