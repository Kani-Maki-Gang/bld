use crate::config::definitions;
use crate::config::OAuth2Info;
use crate::types::{BldError, Result};
use oauth2::basic::BasicClient;
use oauth2::reqwest::http_client;
use oauth2::{AuthorizationCode, CsrfToken, PkceCodeChallenge, TokenResponse};
use std::fs::{create_dir, remove_file, File};
use std::io::{stdin, Write};
use std::path::PathBuf;

fn oauth2_url_summary() {
    println!(
        "Open the printed url in a browser in order to login with the specified oauth2 provider."
    );
    println!();
}

fn oauth2_input_summary() {
    println!();
    println!("After logging in input both the provided code and state here.");
}

fn persist_access_token(server: &str, token: &str) -> Result<()> {
    let mut path = PathBuf::new();
    path.push(definitions::REMOTE_SERVER_OAUTH2);
    if !path.is_dir() {
        create_dir(&path)?;
    }
    path.push(server);
    if path.is_file() {
        remove_file(&path)?;
    }
    let mut handle = File::create(path)?;
    handle.write_all(token.as_bytes())?;
    Ok(())
}

fn stdin_with_label(label: &str) -> Result<String> {
    let mut value = String::new();
    println!("{}: ", label);
    stdin().read_line(&mut value)?;
    Ok(value.trim().to_string())
}

pub trait Login {
    fn login(&self, server: &str) -> Result<String>;
}

impl Login for OAuth2Info {
    fn login(&self, server: &str) -> Result<String> {
        let client = BasicClient::new(
            self.client_id.clone(),
            Some(self.client_secret.clone()),
            self.auth_url.clone(),
            Some(self.token_url.clone()),
        )
        .set_redirect_url(self.redirect_url.clone());
        let (pkce_challenge, _pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let mut auth_url = client
            .authorize_url(CsrfToken::new_random)
            .set_pkce_challenge(pkce_challenge);
        for scope in &self.scopes {
            auth_url = auth_url.add_scope(scope.clone());
        }
        let (auth_url, csrf_token) = auth_url.url();

        oauth2_url_summary();
        println!("{}", auth_url.to_string());
        oauth2_input_summary();

        let code = AuthorizationCode::new(stdin_with_label("code")?);
        let state = CsrfToken::new(stdin_with_label("state")?);
        if state.secret() != csrf_token.secret() {
            let message = String::from("state token not the one expected. operation is aborted");
            return Err(BldError::Other(message));
        }

        let token_res = client.exchange_code(code).request(http_client)?;
        persist_access_token(server, token_res.access_token().secret())?;
        Ok(String::new())
    }
}
