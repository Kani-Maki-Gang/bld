use bld_config::RegistryConfig;
use serde::{Deserialize, Serialize};

#[cfg(feature = "all")]
use anyhow::Result;

#[cfg(feature = "all")]
use crate::token_context::v3::{ApplyContext, ExecutionContext};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Registry {
    FromConfig(String),
    Full(RegistryConfig),
}

#[cfg(feature = "all")]
impl ApplyContext for Registry {
    async fn apply_context<C: ExecutionContext>(&mut self, ctx: &C) -> Result<()> {
        match self {
            Registry::FromConfig(url) => {
                *url = ctx.transform(url.to_owned()).await?;
            }
            Registry::Full(ref mut config) => {
                config.url = ctx.transform(config.url.to_owned()).await?;
                if let Some(ref mut username) = config.username {
                    config.username = Some(ctx.transform(username.to_owned()).await?);
                }
                if let Some(ref mut password) = config.password {
                    config.password = Some(ctx.transform(password.to_owned()).await?);
                }
            }
        }
        Ok(())
    }
}
