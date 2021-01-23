use crate::config::AuthValidation;
use crate::types::{BldError, Result};
use actix_http::error::ErrorUnauthorized;
use actix_service::{Service, Transform};
use actix_web::{client::Client, dev::ServiceRequest, dev::ServiceResponse, Error};
use awc::http::StatusCode;
use futures::future::{ok, Ready};
use futures::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

type StdResult<T, V> = std::result::Result<T, V>;

pub struct UserValidator {
    auth: AuthValidation,
}

impl UserValidator {
    pub fn new(auth: AuthValidation) -> Self {
        Self { auth }
    }
}

impl<S, B> Transform<S> for UserValidator
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = UserValidatorMiddleware<S>;
    type Future = Ready<StdResult<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(UserValidatorMiddleware::new(self.auth.clone(), service))
    }
}

pub struct UserValidatorMiddleware<S> {
    auth: AuthValidation,
    service: S,
}

impl<S> UserValidatorMiddleware<S> {
    pub fn new(auth: AuthValidation, service: S) -> Self {
        Self { auth, service }
    }
}

impl<S, B> Service for UserValidatorMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = StdResult<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<StdResult<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        let auth = self.auth.clone();
        let bearer = get_header(&req, "Authorization");
        let ignore_auth = req.uri().to_string().contains("authRedirect");
        let fut = self.service.call(req);
        Box::pin(async move {
            let mut validation: Result<()> = Ok(());
            if !ignore_auth {
                validation = match auth {
                    AuthValidation::OAuth2(url) => oauth2_validate(&url, &bearer).await,
                    AuthValidation::Ldap => Ok(()),
                    AuthValidation::None => Ok(()),
                };
            }
            if let Ok(()) = validation {
                return Ok(fut.await?);
            }
            Err(ErrorUnauthorized(""))
        })
    }
}

fn get_header(req: &ServiceRequest, name: &str) -> String {
    if let Some(value) = req.headers().get(name) {
        if let Ok(value) = value.to_str() {
            return String::from(value);
        }
    }
    String::new()
}

async fn oauth2_validate(url: &str, bearer: &str) -> Result<()> {
    let client = Client::default();
    let response = client
        .get(url)
        .header("User-Agent", "Bld")
        .header("Authorization", bearer)
        .send()
        .await;
    match response?.status() {
        StatusCode::OK => Ok(()),
        _ => Err(BldError::Other("could not authenticate user".to_string())),
    }
}
