use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::token_context::v2::PipelineContext;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Registry {
    UrlOrConfigKey(String),
    Full {
        url: String,
        username: String,
        password: String,
    },
}

impl Registry {
    pub async fn apply_tokens<'a>(&mut self, context: &PipelineContext<'a>) -> Result<()> {
        match self {
            Registry::UrlOrConfigKey(url) => {
                *url = context.transform(url.to_owned()).await?;
            }
            Registry::Full {
                url,
                username,
                password,
            } => {
                *url = context.transform(url.to_owned()).await?;
                *username = context.transform(username.to_owned()).await?;
                *password = context.transform(password.to_owned()).await?;
            }
        }
        Ok(())
    }
}
