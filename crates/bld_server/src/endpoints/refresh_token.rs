use actix_web::{
    web::{Data, Query},
    HttpResponse, Responder,
};
use anyhow::{anyhow, bail, Result};
use openidconnect::{
    core::CoreClient, reqwest::async_http_client, OAuth2TokenResponse, RefreshToken, TokenResponse,
};
use tracing::error;

use crate::{requests::RefreshTokenParams, responses::TokenResponse as BldTokenResponse};

pub async fn refresh_token(
    info: Query<RefreshTokenParams>,
    client: Data<Option<CoreClient>>,
) -> impl Responder {
    match do_refresh_token(info, client).await {
        Ok(resp) => HttpResponse::Ok().json(resp),
        Err(e) => {
            error!("{e}");
            HttpResponse::BadRequest().body("couldn't refresh access token")
        }
    }
}

async fn do_refresh_token(
    info: Query<RefreshTokenParams>,
    client: Data<Option<CoreClient>>,
) -> Result<BldTokenResponse> {
    let Some(client) = client.get_ref() else {
        bail!("openid core client hasn't been registered");
    };
    let info = info.into_inner();
    let refresh_token = RefreshToken::new(info.refresh_token);

    let token_response = client
        .exchange_refresh_token(&refresh_token)
        .request_async(async_http_client)
        .await?;

    let id_token = token_response
        .id_token()
        .ok_or_else(|| anyhow!("server didn't return an ID token"))?;

    let access_token = token_response.access_token().secret().to_owned();

    let refresh_token = token_response
        .refresh_token()
        .map(|x| x.secret().to_owned());

    Ok(BldTokenResponse::new(access_token, refresh_token))
}
