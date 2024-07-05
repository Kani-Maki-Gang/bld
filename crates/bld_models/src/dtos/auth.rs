use serde::Deserialize;

#[derive(Deserialize)]
pub struct AuthRedirectParams {
    pub code: Option<String>,
    pub error: Option<String>,
    pub state: String,
}
