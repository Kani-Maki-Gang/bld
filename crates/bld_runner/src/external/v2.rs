use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::token_context::v2::PipelineContext;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct External {
    pub name: Option<String>,
    pub server: Option<String>,
    pub pipeline: String,

    #[serde(default)]
    pub variables: HashMap<String, String>,

    #[serde(default)]
    pub environment: HashMap<String, String>,
}

impl External {
    pub fn is(&self, value: &str) -> bool {
        self.name.as_ref().map(|n| n == value).unwrap_or_default() || self.pipeline == value
    }

    pub fn local(pipeline: &str) -> Self {
        Self {
            pipeline: pipeline.to_owned(),
            ..Default::default()
        }
    }

    pub async fn apply_tokens<'a>(&mut self, context: &'a PipelineContext<'a>) -> Result<()> {
        if let Some(name) = self.name.as_mut() {
            *name = context.transform(name.to_owned()).await?;
        }

        if let Some(server) = self.server.as_mut() {
            *server = context.transform(server.to_owned()).await?;
        }

        self.pipeline = context.transform(self.pipeline.to_owned()).await?;

        for (_, v) in self.variables.iter_mut() {
            *v = context.transform(v.to_owned()).await?;
        }

        for (_, v) in self.environment.iter_mut() {
            *v = context.transform(v.to_owned()).await?;
        }

        Ok(())
    }
}
