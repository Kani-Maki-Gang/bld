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
use bld_core::auth::{AuthTokens, LoginProcess};
use bld_sock::messages::{LoginClientMessage, LoginServerMessage};
use openidconnect::{
    core::{CoreAuthenticationFlow, CoreClient},
    reqwest::async_http_client,
    AccessTokenHash, AuthorizationCode, CsrfToken, Nonce, OAuth2TokenResponse, PkceCodeChallenge,
    PkceCodeVerifier, TokenResponse,
};
use tokio::sync::oneshot::{self, error::TryRecvError};
use tracing::error;

pub struct LoginSocket {
    csrf_token: CsrfToken,
    nonce: Nonce,
    pkce_verifier: Option<PkceCodeVerifier>,
    code_rx: Option<oneshot::Receiver<String>>,
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
            pkce_verifier: None,
            nonce,
            code_rx: None,
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
        self.pkce_verifier = Some(verifier);

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
        self.code_rx = Some(code_rx);

        let csrf_token = self.csrf_token.clone();
        let logins = self.logins.clone();
        let login_add_fut =
            async move { logins.add(csrf_token.secret().to_owned(), code_tx).await }
                .into_actor(self)
                .then(|res, _, ctx| {
                    if let Err(e) = res {
                        error!("{e}");
                        ctx.stop();
                    }
                    ready(())
                });
        ctx.wait(login_add_fut);

        let message = LoginServerMessage::AuthorizationUrl(url.to_string());
        ctx.text(serde_json::to_string(&message)?);

        Ok(())
    }

    async fn openid_authorize_code(
        client: Data<Option<CoreClient>>,
        pkce_verifier: PkceCodeVerifier,
        nonce: Nonce,
        code: String,
    ) -> Result<AuthTokens> {
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
            let actual_access_token_hash = AccessTokenHash::from_token(
                token_response.access_token(),
                &id_token.signing_alg()?,
            )?;
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

    fn code_recv_interval(&mut self, ctx: &mut <Self as Actor>::Context) -> Result<()> {
        let Some(code_rx) = self.code_rx.as_mut() else {
            bail!("code receiver hasnt been registered");
        };

        match code_rx.try_recv() {
            Ok(code) => {
                let Some(pkce_verifier) = self.pkce_verifier.take() else {
                    bail!("pkce verifier hasn't been set by the login socket instance");
                };
                let nonce = self.nonce.clone();
                let client = self.client.clone();

                let exchange_code_fut =
                    Self::openid_authorize_code(client, pkce_verifier, nonce, code)
                        .into_actor(self)
                        .then(|res, _, ctx| {
                            let res = res
                                .and_then(|r| {
                                    let message = LoginServerMessage::Completed(r);
                                    serde_json::to_string(&message).map_err(|e| anyhow!(e))
                                })
                                .or_else(|e| {
                                    let message = LoginServerMessage::Failed {
                                        reason: e.to_string(),
                                    };
                                    serde_json::to_string(&message).map_err(|e| anyhow!(e))
                                });
                            match res {
                                Ok(message) => ctx.text(message),
                                Err(e) => ctx.text(e.to_string()),
                            }
                            ctx.stop();
                            ready(())
                        });

                ctx.wait(exchange_code_fut);
            }

            Err(TryRecvError::Closed) => {
                let message = LoginServerMessage::Failed {
                    reason: "unable to verify authentication code".to_string(),
                };
                ctx.text(serde_json::to_string(&message)?);
                ctx.stop();
            }

            Err(TryRecvError::Empty) => {}
        }

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
            let message = LoginServerMessage::Failed {
                reason: "Operation timeout".to_string(),
            };
            if let Ok(text) = serde_json::to_string(&message) {
                ctx.text(text)
            }
            ctx.stop();
        });
        ctx.run_interval(Duration::from_millis(100), |act, ctx| {
            if let Err(e) = act.code_recv_interval(ctx) {
                error!("{e}");
            }
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
            _ => ctx.stop(),
        }
    }
}

pub async fn ws_login(
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
