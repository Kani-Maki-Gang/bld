use actix_web::{
    get,
    web::{Data, Query},
    HttpResponse, Responder,
};
use anyhow::{anyhow, bail, Result};
use bld_config::BldConfig;
use bld_models::{dtos::AuthRedirectParams, login_attempts};
use bld_utils::fs::{AuthTokens, RefreshTokenParams};
use openidconnect::{
    core::CoreClient, reqwest::async_http_client, AccessTokenHash, AuthorizationCode, Nonce,
    OAuth2TokenResponse, PkceCodeVerifier, RefreshToken, TokenResponse,
};
use sea_orm::DatabaseConnection;
use tracing::{error, info};

const AUTH_REDIRECT_SUCCESS: &str =
    "Login completed, you can close this browser tab and go back to your terminal.";
const AUTH_REDIRECT_FAILED: &str = "An error occured while completing the login process.";

async fn openid_authorize_code(
    conn: &DatabaseConnection,
    client: &Option<CoreClient>,
    params: &AuthRedirectParams,
) -> Result<()> {
    if let Some(e) = params.error.as_ref() {
        error!("Error during login process: {e}");
        let _ = login_attempts::update_as_failed_by_csrf_token(conn, &params.state).await;
        bail!("An error occured during the login process");
    }

    let Some(client) = client else {
        let _ = login_attempts::update_as_failed_by_csrf_token(conn, &params.state).await;
        bail!("openid core client hasn't been registered for the server");
    };

    let Some(code) = params.code.to_owned() else {
        let _ = login_attempts::update_as_failed_by_csrf_token(conn, &params.state).await;
        bail!("oidc response code hasn't not provided");
    };

    let login_attempt = login_attempts::select_by_csrf_token(&conn, &params.state).await?;
    let authorization_code = AuthorizationCode::new(code);
    let nonce = Nonce::new(login_attempt.nonce);
    let pkce_verifier = PkceCodeVerifier::new(login_attempt.pkce_verifier);

    let token_response = client
        .exchange_code(authorization_code)
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await;

    let Ok(token_response) = token_response else {
        let _ = login_attempts::update_as_failed_by_csrf_token(conn, &params.state).await;
        bail!("unable to exhange authorization code");
    };

    let id_token = token_response
        .id_token()
        .ok_or_else(|| anyhow!("server didn't return an ID token"))?;

    let claims = id_token.claims(&client.id_token_verifier(), &nonce)?;

    if let Some(access_token_hash) = claims.access_token_hash() {
        let actual_access_token_hash =
            AccessTokenHash::from_token(token_response.access_token(), &id_token.signing_alg()?)?;
        if actual_access_token_hash != *access_token_hash {
            bail!("invalid access token");
        }
    }

    let access_token = token_response.access_token().secret().to_owned();

    let refresh_token = token_response
        .refresh_token()
        .map(|x| x.secret().to_owned());

    login_attempts::update_as_completed_by_csrf_token(
        conn,
        &params.state,
        &access_token,
        refresh_token.as_deref(),
    )
    .await?;

    Ok(())
}

#[get("/v1/auth/available")]
pub async fn available(config: Data<BldConfig>) -> impl Responder {
    if config.local.server.auth.is_some() {
        HttpResponse::Ok().body("")
    } else {
        HttpResponse::BadRequest().body("auth not available")
    }
}

#[get("/v1/auth/redirect")]
pub async fn redirect(
    info: Query<AuthRedirectParams>,
    client: Data<Option<CoreClient>>,
    conn: Data<DatabaseConnection>,
) -> impl Responder {
    info!("Reached handler for /authRedirect route");
    match openid_authorize_code(&conn, client.get_ref(), &info).await {
        Ok(_) => HttpResponse::Ok().body(AUTH_REDIRECT_SUCCESS),
        Err(e) => {
            error!("{e}");
            HttpResponse::Ok().body(AUTH_REDIRECT_FAILED)
        }
    }
}

#[get("/v1/auth/refresh")]
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
