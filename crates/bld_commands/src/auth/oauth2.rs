use std::{
    fmt::Write as FmtWrite,
    fs::{create_dir_all, remove_file, File},
    io::Write,
    path::PathBuf,
    process::ExitStatus,
    time::Duration,
};

use actix::{spawn, System};
use actix_web::{
    get,
    web::{Data, Query},
    App, HttpResponse, HttpServer, Responder,
};
use anyhow::{anyhow, bail, Result};
use bld_config::{definitions::REMOTE_SERVER_OAUTH2, path, Auth, BldConfig};
use bld_server::requests::AuthRedirectInfo;
use bld_utils::{
    sync::IntoData,
    tls::{load_server_certificate, load_server_private_key},
};
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthorizationCode, CsrfToken,
    PkceCodeChallenge, TokenResponse,
};
use rustls::ServerConfig;
use tokio::{
    process::Command,
    sync::mpsc::{channel, Sender},
    time::sleep,
};

fn persist_access_token(server: &str, token: &str) -> Result<()> {
    let mut path = path![REMOTE_SERVER_OAUTH2];

    create_dir_all(&path)?;

    path.push(server);
    if path.is_file() {
        remove_file(&path)?;
    }

    File::create(path)?.write_all(token.as_bytes())?;

    Ok(())
}

async fn do_auth_redirect(
    info: Query<AuthRedirectInfo>,
    config: Data<BldConfig>,
    server: Data<String>,
) -> Result<()> {
    let server = config.server(&server)?;
    let Some(Auth::OAuth2(oauth2)) = &server.auth else {
        bail!("server not configured for oauth2 authentication");
    };

    let client = BasicClient::new(
        oauth2.client_id.clone(),
        Some(oauth2.client_secret.clone()),
        oauth2.auth_url.clone(),
        Some(oauth2.token_url.clone()),
    )
    .set_redirect_uri(oauth2.redirect_url.clone());

    let code = AuthorizationCode::new(info.code.to_owned());

    let token_res = client
        .exchange_code(code)
        .request_async(async_http_client)
        .await
        .map_err(|e| anyhow!(e))?;

    persist_access_token(&server.name, token_res.access_token().secret())?;

    Ok(())
}

#[get("/authRedirect")]
async fn auth_redirect(
    info: Query<AuthRedirectInfo>,
    completion_tx: Data<Sender<Result<()>>>,
    config: Data<BldConfig>,
    server: Data<String>,
) -> impl Responder {
    let result = do_auth_redirect(info, config, server).await;

    spawn(async move {
        sleep(Duration::from_millis(100)).await;
        let _ = completion_tx.send(result).await;
    });

    HttpResponse::Ok()
        .body("Login completed, you can close this browser tab and go back to your terminal.")
}

async fn oauth2_server(
    completion_tx: Data<Sender<Result<()>>>,
    config: Data<BldConfig>,
    server: Data<String>,
) -> Result<()> {
    let host = &config.local.server.host;
    let port = &config.local.server.port;
    let address = format!("{host}:{port}");

    let config_clone = config.clone();

    let mut server = HttpServer::new(move || {
        App::new()
            .app_data(completion_tx.clone())
            .app_data(config_clone.clone())
            .app_data(server.clone())
            .service(auth_redirect)
    });

    server = match &config.local.server.tls {
        Some(tls) => {
            let cert_chain = load_server_certificate(&tls.cert_chain)?;
            let private_key = load_server_private_key(&tls.private_key)?;
            let builder = ServerConfig::builder()
                .with_safe_defaults()
                .with_no_client_auth()
                .with_single_cert(cert_chain, private_key)?;
            server.bind_rustls(address, builder)?
        }
        None => server.bind(address)?,
    };

    server.workers(1).run().await?;

    Ok(())
}

async fn oauth2_prompt(config: Data<BldConfig>, server: Data<String>) -> Result<()> {
    let Some(Auth::OAuth2(oauth2)) = &config.server(&server)?.auth else {
        bail!("server isn't configured for oauth2 authentication");
    };

    let client = BasicClient::new(
        oauth2.client_id.clone(),
        Some(oauth2.client_secret.clone()),
        oauth2.auth_url.clone(),
        Some(oauth2.token_url.clone()),
    )
    .set_redirect_uri(oauth2.redirect_url.clone());

    let (pkce_challenge, _pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let mut auth_url = client
        .authorize_url(CsrfToken::new_random)
        .set_pkce_challenge(pkce_challenge);

    for scope in &oauth2.scopes {
        auth_url = auth_url.add_scope(scope.clone());
    }

    let (auth_url, _) = auth_url.url();

    let mut cmd = Command::new("xdg-open");
    cmd.arg(auth_url.to_string());

    let status = cmd.status().await?;

    let mut message = String::new();
    if !ExitStatus::success(&status) {
        writeln!(message, "Use the below url in order to login:")?;
        writeln!(message)?;
        writeln!(message)?;
        writeln!(message, "{auth_url}")?;
    } else {
        writeln!(
            message,
            "A new browser tab has opened in order to complete the login process."
        )?;
    }

    println!("{message}");

    Ok(())
}

pub fn oauth2_login(config: Data<BldConfig>, server: Data<String>) -> Result<()> {
    System::new().block_on(async move {
        let (tx, mut rx) = channel(4096);
        let tx = tx.into_data();
        let config_clone = config.clone();
        let server_clone = server.clone();

        spawn(async move { oauth2_server(tx, config_clone, server_clone).await });

        oauth2_prompt(config, server).await?;

        rx.recv()
            .await
            .ok_or_else(|| anyhow!("Unable to retrieve login result."))
            .map(|x| {
                if x.is_ok() {
                    println!("Login completed successfully!");
                }
                x
            })?
    })
}
