use serde::Deserialize;

#[derive(Deserialize)]
pub struct AuthRedirectInfo {
    pub code: String,
    pub state: String,
}
