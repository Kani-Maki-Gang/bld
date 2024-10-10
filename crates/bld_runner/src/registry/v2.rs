use bld_config::RegistryConfig;
use serde::{Deserialize, Serialize};

#[cfg(feature = "all")]
use anyhow::Result;

#[cfg(feature = "all")]
use crate::token_context::v2::PipelineContext;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Registry {
    FromConfig(String),
    Full(RegistryConfig),
}

impl Registry {
    #[cfg(feature = "all")]
    pub async fn apply_tokens<'a>(&mut self, context: &PipelineContext<'a>) -> Result<()> {
        match self {
            Registry::FromConfig(url) => {
                *url = context.transform(url.to_owned()).await?;
            }
            Registry::Full(ref mut config) => {
                config.url = context.transform(config.url.to_owned()).await?;
                if let Some(ref mut username) = config.username {
                    config.username = Some(context.transform(username.to_owned()).await?);
                }
                if let Some(ref mut password) = config.password {
                    config.password = Some(context.transform(password.to_owned()).await?);
                }
            }
        }
        Ok(())
    }
}
