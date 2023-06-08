use serde::Deserialize;

#[derive(Deserialize)]
pub struct AuthRedirectParams {
    pub code: String,
    pub state: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RefreshTokenParams {
    pub refresh_token: String,
}
