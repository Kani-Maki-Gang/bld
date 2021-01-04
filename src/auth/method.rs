use crate::config::OAuth2Info;
use crate::types::{BldError, Result};
use oauth2::basic::BasicClient;
use oauth2::reqwest::http_client;
use oauth2::{AuthorizationCode, CsrfToken, PkceCodeChallenge};
use std::io::stdin;

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

fn stdin_with_label(label: &str) -> Result<String> {
    let mut value = String::new();
    println!("{}: ", label);
    stdin().read_line(&mut value)?;
    Ok(value.trim().to_string())
}

pub trait Login {
    fn login(&self) -> Result<String>;
}

impl Login for OAuth2Info {
    fn login(&self) -> Result<String> {
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
        dbg!(&token_res);
        Ok(String::new())
    }
}
