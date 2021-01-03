use serde_derive::Deserialize;

#[derive(Deserialize)]
pub struct AuthRedirectInfo {
    pub code: String,
    pub state: String,
}
