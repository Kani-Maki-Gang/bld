use serde::Deserialize;

#[derive(Deserialize)]
pub struct AuthRedirectParams {
    pub code: String,
    pub state: String,
}
