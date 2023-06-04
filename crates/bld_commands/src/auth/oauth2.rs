use std::{
    fmt::Write as FmtWrite,
    fs::{create_dir_all, remove_file, File},
    io::Write,
    path::PathBuf,
    process::{ExitStatus, Stdio},
    time::Duration,
};

use actix::{spawn, System};
use actix_web::{
    get,
    web::{Data, Query},
    App, HttpResponse, HttpServer, Responder,
};
use anyhow::{anyhow, bail, Result};
use bld_config::{
    definitions::REMOTE_SERVER_OAUTH2, os_name, path, Auth, BldConfig, OSname, OpenIdInfo,
};
use bld_server::requests::AuthRedirectInfo;
use bld_utils::{
    sync::IntoData,
    tls::{load_server_certificate, load_server_private_key},
};
use openidconnect::{
    core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata},
    reqwest::async_http_client,
    AuthorizationCode, CsrfToken, Nonce, OAuth2TokenResponse, PkceCodeChallenge, PkceCodeVerifier,
};
use rustls::ServerConfig;
use tokio::{
    process::Command,
    sync::{
        mpsc::{channel, Sender},
        Mutex,
    },
    time::sleep,
};
use tracing::debug;

const AUTH_REDIRECT_SUCCESS: &str =
    "Login completed, you can close this browser tab and go back to your terminal.";
const AUTH_REDIRECT_FAILED: &str = "An error occured while completing the login process.";

fn persist_token_response(server: &str, token: &str) -> Result<()> {
    let mut path = path![REMOTE_SERVER_OAUTH2];

    create_dir_all(&path)?;

    path.push(server);
    if path.is_file() {
        remove_file(&path)?;
    }

    File::create(path)?.write_all(token.as_bytes())?;

    Ok(())
}

async fn create_core_client(openid: &OpenIdInfo) -> Result<CoreClient> {
    let provider_metadata =
        CoreProviderMetadata::discover_async(openid.issuer_url.clone(), async_http_client).await?;

    let client = CoreClient::from_provider_metadata(
        provider_metadata,
        openid.client_id.clone(),
        Some(openid.client_secret.clone()),
    )
    .set_redirect_uri(openid.redirect_url.clone());

    Ok(client)
}

async fn do_auth_redirect(
    info: Query<AuthRedirectInfo>,
    config: Data<BldConfig>,
    server: Data<String>,
    verifier: Data<Mutex<Option<PkceCodeVerifier>>>,
) -> Result<()> {
    let server = config.server(&server)?;
    let Some(Auth::OpenId(openid)) = &server.auth else {
        bail!("server not configured for oauth2 authentication");
    };

    let client = create_core_client(openid).await?;
    let code = AuthorizationCode::new(info.code.to_owned());

    debug!("finishing login process by verifying the code and state received");

    let verifier = {
        let mut value = verifier.lock().await;
        value
            .take()
            .ok_or_else(|| anyhow!("pkce verifier wasn't provided"))
    }?;

    let token_res = client
        .exchange_code(code)
        .set_pkce_verifier(verifier)
        .request_async(async_http_client)
        .await
        .map_err(|e| {
            dbg!(&e);
            anyhow!(e)
        })?;

    persist_token_response(&server.name, token_res.access_token().secret())?;

    Ok(())
}

#[get("/authRedirect")]
async fn auth_redirect(
    info: Query<AuthRedirectInfo>,
    completion_tx: Data<Sender<Result<()>>>,
    config: Data<BldConfig>,
    server: Data<String>,
    verifier: Data<Mutex<Option<PkceCodeVerifier>>>,
) -> impl Responder {
    let result = do_auth_redirect(info, config, server, verifier).await;

    let response = if result.is_ok() {
        AUTH_REDIRECT_SUCCESS
    } else {
        AUTH_REDIRECT_FAILED
    };

    spawn(async move {
        sleep(Duration::from_millis(100)).await;
        let _ = completion_tx.send(result).await;
    });

    HttpResponse::Ok().body(response)
}

async fn oauth2_server(
    completion_tx: Data<Sender<Result<()>>>,
    config: Data<BldConfig>,
    server: Data<String>,
    verifier: Data<Mutex<Option<PkceCodeVerifier>>>,
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
            .app_data(verifier.clone())
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

async fn oauth2_prompt(
    config: Data<BldConfig>,
    server: Data<String>,
    challenge: PkceCodeChallenge,
) -> Result<()> {
    let Some(Auth::OpenId(openid)) = &config.server(&server)?.auth else {
        bail!("server isn't configured for oauth2 authentication");
    };

    let client = create_core_client(openid).await?;

    let mut auth_url = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .set_pkce_challenge(challenge);

    for scope in &openid.scopes {
        auth_url = auth_url.add_scope(scope.clone());
    }

    let (auth_url, _, _) = auth_url.url();

    let mut command = match os_name() {
        OSname::Linux => {
            debug!("creating xdg-open command for the url {auth_url}");
            let mut cmd = Command::new("xdg-open");
            cmd.arg(auth_url.to_string());
            cmd.stdout(Stdio::null());
            cmd.stderr(Stdio::null());
            cmd
        }
        _ => unimplemented!(),
    };

    println!("A new browser tab will open in order to complete the login process.");

    debug!("launching browser and wating for status");
    let status = command.status().await?;

    let mut message = String::new();
    if !ExitStatus::success(&status) {
        debug!("browser launch failed with status code {status}");
        writeln!(
            message,
            "Couldn't open the browser, please use the below url in order to login:"
        )?;
        write!(message, "{auth_url}")?;
        println!("{message}");
    }

    Ok(())
}

pub fn oauth2_login(config: Data<BldConfig>, server: Data<String>) -> Result<()> {
    System::new().block_on(async move {
        let (compl_tx, mut compl_rx) = channel(4096);
        let compl_tx = compl_tx.into_data();
        let config_clone = config.clone();
        let server_clone = server.clone();

        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let pkce_verifier = Mutex::new(Some(pkce_verifier)).into_data();

        debug!("spawing server task for authentication");
        spawn(
            async move { oauth2_server(compl_tx, config_clone, server_clone, pkce_verifier).await },
        );

        debug!("spawing prompt task for launching login page in the browser");
        spawn(async move {
            let _ = oauth2_prompt(config, server, pkce_challenge).await;
        });

        compl_rx
            .recv()
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
