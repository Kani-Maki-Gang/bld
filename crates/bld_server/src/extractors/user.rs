use actix_web::dev::Payload;
use actix_web::error::ErrorUnauthorized;
use actix_web::http::header::HeaderValue;
use actix_web::web::Data;
use actix_web::{Error, FromRequest, HttpRequest};
use anyhow::{anyhow, bail, Result};
use bld_config::{Auth, BldConfig, UserInfoProperty};
use futures::Future;
use futures_util::future::FutureExt;
use openidconnect::core::{CoreClient, CoreUserInfoClaims};
use openidconnect::reqwest::async_http_client;
use openidconnect::AccessToken;
use std::pin::Pin;

#[derive(Debug)]
pub struct User {
    pub name: String,
}

impl User {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl FromRequest for User {
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let config = req.app_data::<Data<BldConfig>>().cloned();
        let client = req.app_data::<Data<Option<CoreClient>>>().cloned();
        let access_token = get_access_token(req);

        async move {
            let config = config.unwrap();
            let client = client.unwrap();
            if let Some(Auth::OpenId(openid)) = &config.get_ref().local.server.auth {
                return openid_validate(client.as_ref(), access_token, &openid.user_property)
                    .await
                    .map_err(|e| ErrorUnauthorized(e.to_string()));
            }
            Ok(User::new(""))
        }
        .boxed_local()
    }
}

fn get_access_token(request: &HttpRequest) -> AccessToken {
    let bearer = request
        .headers()
        .get("Authorization")
        .unwrap_or(&HeaderValue::from_static(""))
        .to_str()
        .unwrap()
        .replace("Bearer ", "");
    AccessToken::new(bearer)
}

async fn openid_validate(
    client: &Option<CoreClient>,
    access_token: AccessToken,
    user_prop: &UserInfoProperty,
) -> Result<User> {
    let Some(client) = client else {
        bail!("openid core client not registered");
    };

    let res: CoreUserInfoClaims = client
        .user_info(access_token, None)?
        .request_async(async_http_client)
        .await
        .map_err(|e| {
            dbg!(&e);
            e
        })?;

    let user = match user_prop {
        UserInfoProperty::Name => res.name().and_then(|x| x.get(None).map(|n| n.as_str())),
        UserInfoProperty::Email => res.email().map(|e| e.as_str()),
    };

    let user = user
        .ok_or_else(|| anyhow!("couldn't retrieve the user property for the user info response"))?;

    Ok(User::new(user))
}
