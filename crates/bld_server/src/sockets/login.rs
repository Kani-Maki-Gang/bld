use std::time::{Duration, Instant};

use actix_web::{
    HttpRequest, Responder,
    rt::{spawn, time},
    web::{Data, Payload},
};
use actix_ws::{CloseCode, CloseReason, Message, Session, handle};
use anyhow::{Result, bail};
use bld_config::{Auth, BldConfig};
use bld_models::{
    dtos::{AuthTokens, LoginClientMessage, LoginServerMessage},
    login_attempts::{self, InsertLoginAttempt, LoginAttemptStatus},
};
use chrono::Utc;
use futures::StreamExt;
use openidconnect::{
    CsrfToken, Nonce, PkceCodeChallenge,
    core::{CoreAuthenticationFlow, CoreClient},
};
use sea_orm::DatabaseConnection;
use tracing::{error, warn};

const STATUS_CHECK_INTERVAL_MS: u64 = 500;
const HEARTBEAT_INTERVAL_MS: u64 = 5_000;
const CLIENT_TIMEOUT_MS: u64 = 15_000;

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

    async fn openid_authorization_url(&mut self, session: &mut Session) -> Result<()> {
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

        let model = InsertLoginAttempt {
            csrf_token: csrf_token.secret().to_owned(),
            nonce: nonce.secret().to_owned(),
            pkce_verifier: verifier.secret().to_owned(),
        };

        if let Err(e) = login_attempts::insert(&conn, model).await {
            let message = serde_json::to_string(&LoginServerMessage::Failed(e.to_string()))?;
            session.text(message).await?;
        }

        let message = LoginServerMessage::AuthorizationUrl(url.to_string());
        session.text(serde_json::to_string(&message)?).await?;

        Ok(())
    }

    async fn handle_client_message(&mut self, session: &mut Session, message: &str) -> Result<()> {
        let _: LoginClientMessage = serde_json::from_str(message)?;

        match &self.config.as_ref().local.server.auth {
            Some(Auth::OpenId(_)) => self.openid_authorization_url(session).await?,
            _ => bail!("no authentication method configured for server"),
        }

        Ok(())
    }

    async fn check_status(&self, session: &mut Session) -> bool {
        let csrf_token = self.csrf_token.secret().to_owned();
        let conn = self.conn.clone();

        let Ok(login_attempt) = login_attempts::select_by_csrf_token(&conn, &csrf_token).await
        else {
            let message = LoginServerMessage::Failed("Login operation failed".to_string());
            if let Ok(text) = serde_json::to_string(&message) {
                let _ = session.text(text).await.inspect_err(|e| error!("{e}"));
            }
            return false;
        };

        let status: Result<LoginAttemptStatus> = login_attempt.status.try_into();

        if let Ok(LoginAttemptStatus::Completed) = status {
            let Some(access_token) = login_attempt.access_token else {
                let message = LoginServerMessage::Failed("Access token not found".to_string());
                if let Ok(text) = serde_json::to_string(&message) {
                    let _ = session.text(text).await.inspect_err(|e| error!("{e}"));
                }
                return false;
            };

            let auth_tokens = AuthTokens::new(access_token, login_attempt.refresh_token);
            let message = LoginServerMessage::Completed(auth_tokens);
            if let Ok(text) = serde_json::to_string(&message) {
                let _ = session.text(text).await.inspect_err(|e| error!("{e}"));
            }

            return false;
        }

        if let Ok(LoginAttemptStatus::Failed) = status {
            let message = LoginServerMessage::Failed("Login operation failed".to_string());
            if let Ok(text) = serde_json::to_string(&message) {
                let _ = session.text(text).await.inspect_err(|e| error!("{e}"));
            }
            return false;
        }

        if Utc::now().naive_utc() > login_attempt.date_expires {
            let message = LoginServerMessage::Failed("Operation timeout".to_string());
            if let Ok(text) = serde_json::to_string(&message) {
                let _ = session.text(text).await.inspect_err(|e| error!("{e}"));
            }
            return false;
        }

        true
    }

    async fn cleanup(&mut self) {
        if let Err(e) =
            login_attempts::delete_by_csrf_token(self.conn.as_ref(), self.csrf_token.secret()).await
        {
            error!("{e}");
        }
    }
}

pub async fn ws(
    req: HttpRequest,
    body: Payload,
    config: Data<BldConfig>,
    client: Data<Option<CoreClient>>,
    conn: Data<DatabaseConnection>,
) -> actix_web::Result<impl Responder> {
    let csrf_token = CsrfToken::new_random();
    let nonce = Nonce::new_random();
    let mut socket = LoginSocket::new(csrf_token, nonce, config, client, conn);
    let (response, mut session, mut msg_stream) = handle(&req, body)?;

    spawn(async move {
        let mut reason: Option<CloseReason> = None;
        let mut interval = time::interval(Duration::from_millis(STATUS_CHECK_INTERVAL_MS));
        let mut hb_interval = time::interval(Duration::from_millis(HEARTBEAT_INTERVAL_MS));
        let mut last_pong = Instant::now();

        loop {
            tokio::select! {
                Some(msg) = msg_stream.next() => {
                    let Ok(msg) = msg.inspect_err(|e| error!("{e}")) else {
                        break;
                    };
                    match msg {
                        Message::Text(txt) => {
                            if let Err(e) = socket.handle_client_message(&mut session, &txt).await {
                                reason = Some(CloseCode::Error.into());
                                error!("{e}");
                                break;
                            }
                        }

                        Message::Ping(msg) => {
                            if let Err(e) = session.pong(&msg).await {
                                reason = Some(CloseCode::Error.into());
                                error!("{e}");
                                break;
                            }
                        }

                        Message::Pong(_) => {
                            last_pong = Instant::now();
                        }

                        Message::Continuation(_) | Message::Nop => {}

                        Message::Close(r) => {
                            reason = r;
                            break;
                        }

                        _ => break,
                    }
                }

                _ = interval.tick() => {
                    if !socket.check_status(&mut session).await {
                        break;
                    }
                }

                _ = hb_interval.tick() => {
                    if Instant::now().duration_since(last_pong)
                        > Duration::from_millis(CLIENT_TIMEOUT_MS)
                    {
                        warn!("client heartbeat timed out, closing session");
                        reason = Some(CloseCode::Away.into());
                        break;
                    }
                    if let Err(e) = session.ping(b"").await {
                        error!("ping failed: {e}");
                        reason = Some(CloseCode::Error.into());
                        break;
                    }
                }
            }
        }

        if let Err(e) = session.close(reason).await {
            error!("encountered error while closing websocket session due to {e}");
        }

        socket.cleanup().await
    });

    Ok(response)
}
