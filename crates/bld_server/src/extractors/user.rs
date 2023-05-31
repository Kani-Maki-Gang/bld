use actix_web::dev::Payload;
use actix_web::error::ErrorUnauthorized;
use actix_web::http::header::HeaderValue;
use actix_web::web::Data;
use actix_web::{Error, FromRequest, HttpRequest};
use anyhow::{anyhow, Result};
use bld_config::{AuthValidation, BldConfig};
use bld_utils::request::Request;
use futures::Future;
use futures_util::future::FutureExt;
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
            if let Some(AuthValidation::OAuth2 {
                user_info_url,
                user_info_property,
            }) = &config.get_ref().local.server.auth
            {
                return oauth2_validate(user_info_url, user_info_property, bearer)
                    .await
                    .map_err(|e| ErrorUnauthorized(e.to_string()));
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

async fn oauth2_validate(url: &str, pointer: &str, bearer: String) -> Result<User> {
    let response: serde_json::Value = Request::get(url)
        .header("Authorization", &bearer)
        .send()
        .await
        .map_err(|e| {
            error!("authorization check failed to remote server with: {}", e);
            anyhow!("could not authenticate user")
        })?;
    let user_id = response
        .pointer(pointer)
        .ok_or_else(|| anyhow!("user property not found"))?;
    Ok(User::new(&user_id.to_string()))
}
