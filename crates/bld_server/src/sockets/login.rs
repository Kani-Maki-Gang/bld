use std::time::Duration;

use actix::{
    fut::{future::ActorFutureExt, ready},
    Actor, ActorContext, AsyncContext, StreamHandler, WrapFuture,
};
use actix_web::{
    web::{Data, Payload},
    Error, HttpRequest, HttpResponse,
};
use actix_web_actors::ws::{start, Message, ProtocolError, WebsocketContext};
use anyhow::{anyhow, bail, Result};
use bld_config::{Auth, BldConfig};
use bld_core::auth::LoginProcess;
use bld_models::dtos::{LoginClientMessage, LoginServerMessage};
use bld_utils::fs::AuthTokens;
use openidconnect::{
    core::{CoreAuthenticationFlow, CoreClient},
    reqwest::async_http_client,
    AccessTokenHash, AuthorizationCode, CsrfToken, Nonce, OAuth2TokenResponse, PkceCodeChallenge,
    PkceCodeVerifier, TokenResponse,
};
use tokio::sync::oneshot;
use tracing::error;

pub struct LoginSocket {
    csrf_token: CsrfToken,
    nonce: Nonce,
    config: Data<BldConfig>,
    client: Data<Option<CoreClient>>,
    logins: Data<LoginProcess>,
}

impl LoginSocket {
    pub fn new(
        csrf_token: CsrfToken,
        nonce: Nonce,
        config: Data<BldConfig>,
        client: Data<Option<CoreClient>>,
        logins: Data<LoginProcess>,
    ) -> Self {
        Self {
            csrf_token,
            nonce,
            config,
            client,
            logins,
        }
    }

    fn openid_authorization_url(&mut self, ctx: &mut <Self as Actor>::Context) -> Result<()> {
        let Some(client) = self.client.get_ref() else {
            bail!("openid core client hasn't be registered for server");
        };

        let Some(Auth::OpenId(openid)) = &self.config.get_ref().local.server.auth else {
            bail!("openid authentication method not registered for server");
        };

        let csrf_token = self.csrf_token.clone();
        let nonce = self.nonce.clone();

        let state_fn = || csrf_token;
        let nonce_fn = || nonce;

        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();

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

        let (code_tx, code_rx) = oneshot::channel();

        let csrf_token = self.csrf_token.clone();
        let client = self.client.clone();
        let nonce = self.nonce.clone();
        let logins = self.logins.clone();

        let login_fut = async move {
            logins.add(csrf_token.secret().to_owned(), code_tx).await?;
            let code_res = openid_authorize_code(code_rx, client, verifier, nonce).await;
            let message = match code_res {
                Ok(tokens) => serde_json::to_string(&LoginServerMessage::Completed(tokens))?,
                Err(e) => serde_json::to_string(&LoginServerMessage::Failed(e.to_string()))?,
            };
            Ok(message)
        }
        .into_actor(self)
        .then(|res: Result<String>, _, ctx| {
            match res {
                Ok(msg) => {
                    ctx.text(msg);
                }
                Err(e) => {
                    error!("{e}");
                    ctx.stop();
                }
            }
            ready(())
        });
        ctx.spawn(login_fut);

        let message = LoginServerMessage::AuthorizationUrl(url.to_string());
        ctx.text(serde_json::to_string(&message)?);

        Ok(())
    }

    fn handle_client_message(
        &mut self,
        ctx: &mut <Self as Actor>::Context,
        message: &str,
    ) -> Result<()> {
        let _: LoginClientMessage = serde_json::from_str(message)?;

        match &self.config.as_ref().local.server.auth {
            Some(Auth::OpenId(_)) => self.openid_authorization_url(ctx)?,
            _ => bail!("no authentication method configured for server"),
        }

        Ok(())
    }
}

impl Actor for LoginSocket {
    type Context = WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_later(Duration::from_secs(600), |_, ctx| {
            let message = LoginServerMessage::Failed("Operation timeout".to_string());
            if let Ok(text) = serde_json::to_string(&message) {
                ctx.text(text)
            }
            ctx.stop();
        });
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {
        let token = self.csrf_token.secret().to_owned();
        let logins = self.logins.clone();
        let logins_remove_fut = async move { logins.remove(token).await }
            .into_actor(self)
            .then(|res, _, _| {
                if let Err(e) = res {
                    error!("{e}");
                }
                ready(())
            });
        ctx.wait(logins_remove_fut);
    }
}

impl StreamHandler<Result<Message, ProtocolError>> for LoginSocket {
    fn handle(&mut self, item: Result<Message, ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(Message::Text(txt)) => {
                if let Err(e) = self.handle_client_message(ctx, &txt) {
                    error!("{e}");
                    ctx.stop();
                }
            }
            Ok(Message::Ping(msg)) => {
                ctx.pong(&msg);
            }
            Ok(Message::Pong(msg)) => {
                ctx.ping(&msg);
            }
            Ok(Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => {}
        }
    }
}

async fn openid_authorize_code(
    code_rx: oneshot::Receiver<String>,
    client: Data<Option<CoreClient>>,
    pkce_verifier: PkceCodeVerifier,
    nonce: Nonce,
) -> Result<AuthTokens> {
    let Ok(code) = code_rx.await else {
        bail!("unable to verify authentication code");
    };

    let Some(client) = client.get_ref() else {
        bail!("openid core client hasn't been registered for the server");
    };

    let authorization_code = AuthorizationCode::new(code);

    let token_response = client
        .exchange_code(authorization_code)
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await;

    let Ok(token_response) = token_response else {
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

    Ok(AuthTokens::new(access_token, refresh_token))
}

pub async fn ws(
    req: HttpRequest,
    stream: Payload,
    config: Data<BldConfig>,
    client: Data<Option<CoreClient>>,
    logins: Data<LoginProcess>,
) -> Result<HttpResponse, Error> {
    let csrf_token = CsrfToken::new_random();
    let nonce = Nonce::new_random();
    let socket = LoginSocket::new(csrf_token, nonce, config, client, logins);
    start(socket, &req, stream)
}
