use actix_web::{
    get,
    web::{Data, Query},
    HttpResponse, Responder,
};
use anyhow::{bail, Result};
use bld_core::auth::Logins;
use bld_models::dtos::AuthRedirectParams;
use bld_utils::fs::{AuthTokens, RefreshTokenParams};
use openidconnect::{
    core::CoreClient, reqwest::async_http_client, OAuth2TokenResponse, RefreshToken,
};
use tracing::{error, info};

const AUTH_REDIRECT_SUCCESS: &str =
    "Login completed, you can close this browser tab and go back to your terminal.";
const AUTH_REDIRECT_FAILED: &str = "An error occured while completing the login process.";

#[get("/auth/redirect")]
pub async fn redirect(
    info: Query<AuthRedirectParams>,
    logins: Data<Logins>,
) -> impl Responder {
    info!("Reached handler for /authRedirect route");
    let code = info.code.to_owned();
    let token = info.state.to_owned();
    match logins.code(token, code).await {
        Ok(_) => HttpResponse::Ok().body(AUTH_REDIRECT_SUCCESS),
        Err(e) => {
            error!("{e}");
            HttpResponse::Ok().body(AUTH_REDIRECT_FAILED)
        }
    }
}

#[get("/auth/refresh")]
pub async fn refresh(
    info: Query<RefreshTokenParams>,
    client: Data<Option<CoreClient>>,
) -> impl Responder {
    match do_auth_refresh(info, client).await {
        Ok(resp) => HttpResponse::Ok().json(resp),
        Err(e) => {
            error!("{e}");
            HttpResponse::BadRequest().body("couldn't refresh access token")
        }
    }
}

async fn do_auth_refresh(
    info: Query<RefreshTokenParams>,
    client: Data<Option<CoreClient>>,
) -> Result<AuthTokens> {
    let Some(client) = client.get_ref() else {
        bail!("openid core client hasn't been registered");
    };
    let info = info.into_inner();
    let refresh_token = RefreshToken::new(info.refresh_token);

    let token_response = client
        .exchange_refresh_token(&refresh_token)
        .request_async(async_http_client)
        .await?;

    let access_token = token_response.access_token().secret().to_owned();

    let refresh_token = token_response
        .refresh_token()
        .map(|x| x.secret().to_owned());

    Ok(AuthTokens::new(access_token, refresh_token))
}
