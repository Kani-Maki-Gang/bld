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
use anyhow::{bail, Result};
use bld_config::{Auth, BldConfig};
use bld_models::{
    dtos::{LoginClientMessage, LoginServerMessage},
    login_attempts::{self, InsertLoginAttempt, LoginAttemptStatus},
};
use bld_utils::fs::AuthTokens;
use chrono::Utc;
use openidconnect::{
    core::{CoreAuthenticationFlow, CoreClient},
    CsrfToken, Nonce, PkceCodeChallenge,
};
use sea_orm::DatabaseConnection;
use tracing::error;

pub struct LoginSocket {
    csrf_token: CsrfToken,
    nonce: Nonce,
    config: Data<BldConfig>,
    client: Data<Option<CoreClient>>,
    conn: Data<DatabaseConnection>,
}

impl LoginSocket {
    pub fn new(
        csrf_token: CsrfToken,
        nonce: Nonce,
        config: Data<BldConfig>,
        client: Data<Option<CoreClient>>,
        conn: Data<DatabaseConnection>,
    ) -> Self {
        Self {
            csrf_token,
            nonce,
            config,
            client,
            conn,
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

        let csrf_token = self.csrf_token.clone();
        let nonce = self.nonce.clone();
        let conn = self.conn.clone();

        let login_fut = async move {
            let model = InsertLoginAttempt {
                csrf_token: csrf_token.secret().to_owned(),
                nonce: nonce.secret().to_owned(),
                pkce_verifier: verifier.secret().to_owned(),
            };
            login_attempts::insert(&conn, model).await
        }
        .into_actor(self)
        .then(|res: Result<()>, _, ctx| {
            if let Err(e) = res {
                let message = serde_json::to_string(&LoginServerMessage::Failed(e.to_string()));
                if let Ok(message) = message {
                    ctx.text(message);
                } else {
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

    fn check_status(act: &Self, ctx: &mut <Self as Actor>::Context) {
        let csrf_token = act.csrf_token.secret().to_owned();
        let conn = act.conn.clone();
        let status_fut =
            async move { login_attempts::select_by_csrf_token(&conn, &csrf_token).await }
                .into_actor(act)
                .then(|res, _, ctx| {
                    let Ok(login_attempt) = res else {
                        let message =
                            LoginServerMessage::Failed("Login operation failed".to_string());
                        if let Ok(text) = serde_json::to_string(&message) {
                            ctx.text(text);
                        }
                        return ready(());
                    };

                    let status: Result<LoginAttemptStatus> = login_attempt.status.try_into();

                    if let Ok(LoginAttemptStatus::Completed) = status {
                        let Some(access_token) = login_attempt.access_token else {
                            let message =
                                LoginServerMessage::Failed("Access token not found".to_string());
                            if let Ok(text) = serde_json::to_string(&message) {
                                ctx.text(text);
                            }
                            return ready(());
                        };

                        let auth_tokens =
                            AuthTokens::new(access_token, login_attempt.refresh_token);
                        let message = LoginServerMessage::Completed(auth_tokens);
                        if let Ok(text) = serde_json::to_string(&message) {
                            ctx.text(text);
                        }

                        return ready(());
                    }

                    if let Ok(LoginAttemptStatus::Failed) = status {
                        let message =
                            LoginServerMessage::Failed("Login operation failed".to_string());
                        if let Ok(text) = serde_json::to_string(&message) {
                            ctx.text(text);
                        }
                        return ready(());
                    }

                    if Utc::now().naive_utc() > login_attempt.date_expires {
                        let message = LoginServerMessage::Failed("Operation timeout".to_string());
                        if let Ok(text) = serde_json::to_string(&message) {
                            ctx.text(text);
                        }
                        ctx.stop();
                        return ready(());
                    }

                    return ready(());
                });
        ctx.spawn(status_fut);
    }

    fn cleanup(&mut self, ctx: &mut <LoginSocket as Actor>::Context) {
        let token = self.csrf_token.secret().to_owned();
        let conn = self.conn.clone();
        let logins_remove_fut =
            async move { login_attempts::delete_by_csrf_token(&conn, &token).await }
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

impl Actor for LoginSocket {
    type Context = WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(Duration::from_secs(1), |act, ctx| {
            LoginSocket::check_status(act, ctx);
        });
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {
        self.cleanup(ctx);
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

pub async fn ws(
    req: HttpRequest,
    stream: Payload,
    config: Data<BldConfig>,
    client: Data<Option<CoreClient>>,
    conn: Data<DatabaseConnection>,
) -> Result<HttpResponse, Error> {
    let csrf_token = CsrfToken::new_random();
    let nonce = Nonce::new_random();
    let socket = LoginSocket::new(csrf_token, nonce, config, client, conn);
    start(socket, &req, stream)
}
