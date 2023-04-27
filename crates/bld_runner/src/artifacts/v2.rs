use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::token_context::v2::PipelineContext;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Artifacts {
    pub method: String,
    pub from: String,
    pub to: String,
    pub ignore_errors: Option<bool>,
    pub after: Option<String>,
}

impl Artifacts {
    pub async fn apply_tokens<'a>(&mut self, context: &'a PipelineContext<'a>) -> Result<()> {
        self.from = context.transform(self.from.to_owned()).await?;
        self.to = context.transform(self.to.to_owned()).await?;

        if let Some(after) = self.after.as_mut() {
            self.after = Some(context.transform(after.to_owned()).await?);
        }

        Ok(())
    }
}
