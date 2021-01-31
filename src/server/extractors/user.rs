use crate::config::{AuthValidation, BldConfig};
use crate::types::{BldError, Result};
use actix_http::error::ErrorUnauthorized;
use actix_web::{Error, HttpRequest, FromRequest};
use actix_web::client::Client;
use actix_web::dev::Payload;
use actix_web::http::HeaderValue;
use actix_web::web::Data;
use awc::http::StatusCode;
use futures::Future;
use futures_util::future::FutureExt;
use std::pin::Pin;

type StdResult<T, V> = std::result::Result<T, V>;

#[derive(Debug)]
pub struct User {
    pub name: String
}

impl User {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }
}

impl FromRequest for User {
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = StdResult<Self, Self::Error>>>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let config = req.app_data::<Data<BldConfig>>().unwrap().clone();
        let bearer = get_bearer(&req);
        async move {
            if let AuthValidation::OAuth2(url) = &config.get_ref().local.auth {
                return match oauth2_validate(&url, &bearer).await {
                    Ok(user) => Ok(user),
                    Err(_) => Err(ErrorUnauthorized("")),
                };
            }
            Ok(User::new(""))
        }.boxed_local()
    }
}

fn get_bearer(request: &HttpRequest) -> String {
    request
        .headers()
        .get("Authorization")
        .or(Some(&HeaderValue::from_static("")))
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

async fn oauth2_validate(url: &str, bearer: &str) -> Result<User> {
    let client = Client::default();
    let mut resp = client
        .get(url)
        .header("User-Agent", "Bld")
        .header("Authorization", bearer)
        .send()
        .await;
    let body = resp.as_mut()?.body().await?;
    match resp?.status() {
        StatusCode::OK => {
            let body = String::from_utf8_lossy(&body).to_string();
            let value: serde_json::Value = serde_json::from_str(&body)?;
            Ok(User::new(&value["login"].to_string()))
        }
        _ => Err(BldError::Other("could not authenticate user".to_string())),
    }
}
