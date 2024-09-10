use actix_web::{
    get,
    web::{Data, Query},
    HttpResponse, Responder,
};
use anyhow::{anyhow, bail, Result};
use bld_config::{Auth, BldConfig};
use bld_models::{
    dtos::{AuthRedirectParams, AuthTokens, RefreshTokenParams},
    login_attempts::{self, InsertLoginAttempt},
};
use chrono::Utc;
use openidconnect::{
    core::{CoreAuthenticationFlow, CoreClient},
    reqwest::async_http_client,
    url::Url,
    AccessTokenHash, AuthorizationCode, CsrfToken, Nonce, OAuth2TokenResponse, PkceCodeChallenge,
    PkceCodeVerifier, RefreshToken, TokenResponse,
};
use sea_orm::DatabaseConnection;
use tracing::{error, info};

const AUTH_REDIRECT_SUCCESS: &str =
    "Login completed, you can close this browser tab and go back to your terminal.";
const AUTH_REDIRECT_FAILED: &str = "An error occured while completing the login process.";

pub struct WebCoreClient(pub Option<CoreClient>);

async fn openid_authorize_url(
    client: &CoreClient,
    config: &BldConfig,
    csrf_token: &CsrfToken,
    nonce: &Nonce,
    challenge: PkceCodeChallenge,
) -> Result<Url> {
    let Some(Auth::OpenId(openid)) = &config.local.server.auth else {
        bail!("openid authentication method not registered for server");
    };

    let csrf_token = csrf_token.clone();
    let nonce = nonce.clone();

    let state_fn = || csrf_token;
    let nonce_fn = || nonce;

    let mut auth_url = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            state_fn,
            nonce_fn,
        )
        .set_pkce_challenge(challenge);

    for scope in openid.scopes.iter() {
        auth_url = auth_url.add_scope(scope.clone());
    }

    let (url, _, _) = auth_url.url();
    Ok(url)
}

async fn openid_authorize_code(
    conn: &DatabaseConnection,
    client: &Option<CoreClient>,
    params: &AuthRedirectParams,
) -> Result<AuthTokens> {
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

    let login_attempt = login_attempts::select_by_csrf_token(conn, &params.state).await?;

    if login_attempt.date_expires < Utc::now().naive_utc() {
        let _ = login_attempts::update_as_failed_by_csrf_token(conn, &params.state).await;
        bail!("Login failed. Operation timeout");
    }

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

    Ok(AuthTokens {
        access_token,
        refresh_token,
    })
}

#[get("/v1/auth/available")]
pub async fn available(config: Data<BldConfig>) -> impl Responder {
    info!("Reached handler for /v1/auth/available route");
    if config.local.server.auth.is_some() {
        HttpResponse::Ok().body("")
    } else {
        HttpResponse::BadRequest().body("auth not available")
    }
}

#[get("/v1/auth/web-client/start")]
pub async fn web_client_start(
    config: Data<BldConfig>,
    web_core_client: Data<WebCoreClient>,
    conn: Data<DatabaseConnection>,
) -> impl Responder {
    info!("Reached handler for /v1/auth/web-client/start route");
    let WebCoreClient(Some(web_core_client)) = web_core_client.get_ref() else {
        return HttpResponse::BadRequest().body("");
    };

    let csrf_token = CsrfToken::new_random();
    let nonce = Nonce::new_random();
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
    let url_result =
        openid_authorize_url(web_core_client, &config, &csrf_token, &nonce, challenge).await;

    if let Err(e) = url_result {
        error!("Error during creation of authorization url due to {e}");
        return HttpResponse::BadRequest().body("");
    }
    let authorization_url = url_result.unwrap();

    let login_attempt = InsertLoginAttempt {
        nonce: nonce.secret().to_owned(),
        csrf_token: csrf_token.secret().to_owned(),
        pkce_verifier: verifier.secret().to_owned(),
    };

    if login_attempts::insert(conn.get_ref(), login_attempt)
        .await
        .is_err()
    {
        return HttpResponse::Found()
            .append_header(("Location", authorization_url.to_string()))
            .finish();
    }

    HttpResponse::BadRequest().body("")
}

#[get("/v1/auth/web-client/validate")]
pub async fn web_client_validate(
    info: Query<AuthRedirectParams>,
    client: Data<WebCoreClient>,
    conn: Data<DatabaseConnection>,
) -> impl Responder {
    info!("Reached handler for /v1/auth/web-client/validate route");
    match openid_authorize_code(&conn, &client.get_ref().0, &info).await {
        Ok(tokens) => HttpResponse::Ok().json(tokens),
        Err(e) => {
            error!("{e}");
            HttpResponse::Ok().body(AUTH_REDIRECT_FAILED)
        }
    }
}

#[get("/v1/auth/redirect")]
pub async fn redirect(
    info: Query<AuthRedirectParams>,
    client: Data<Option<CoreClient>>,
    conn: Data<DatabaseConnection>,
) -> impl Responder {
    info!("Reached handler for /v1/auth/redirect route");
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
    info!("Reached handler for /v1/auth/refresh route");
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

    Ok(AuthTokens {
        access_token,
        refresh_token,
    })
}
