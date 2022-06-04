use actix_web::dev::Payload;
use actix_web::error::ErrorUnauthorized;
use actix_web::http::header::HeaderValue;
use actix_web::web::Data;
use actix_web::{Error, FromRequest, HttpRequest};
use anyhow::anyhow;
use bld_config::{AuthValidation, BldConfig};
use bld_utils::request;
use futures::Future;
use futures_util::future::FutureExt;
use std::collections::HashMap;
use std::pin::Pin;
use tracing::error;

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
        let config = req.app_data::<Data<BldConfig>>().unwrap().clone();
        let bearer = get_bearer(req);
        async move {
            if let AuthValidation::OAuth2(url) = &config.get_ref().local.auth {
                return match oauth2_validate(url.to_string(), bearer).await {
                    Ok(user) => Ok(user),
                    Err(_) => Err(ErrorUnauthorized("")),
                };
            }
            Ok(User::new(""))
        }
        .boxed_local()
    }
}

fn get_bearer(request: &HttpRequest) -> String {
    request
        .headers()
        .get("Authorization")
        .unwrap_or(&HeaderValue::from_static(""))
        .to_str()
        .unwrap()
        .to_string()
}

async fn oauth2_validate(url: String, bearer: String) -> anyhow::Result<User> {
    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), bearer);
    let res = request::get(url.to_string(), headers).await.map_err(|e| {
        error!("authorization check failed to remote server with: {}", e);
        anyhow!("could not authenticate user")
    })?;
    let value: serde_json::Value = serde_json::from_str(&res)?;
    Ok(User::new(&value["login"].to_string()))
}
