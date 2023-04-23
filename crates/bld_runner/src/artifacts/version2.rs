use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
    pipeline::traits::{ApplyTokens, CompleteTokenTransformer},
    token_context::version2::PipelineContext,
};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Artifacts {
    pub method: String,
    pub from: String,
    pub to: String,
    pub ignore_errors: Option<bool>,
    pub after: Option<String>,
}

#[async_trait]
impl<'a> ApplyTokens<'a, PipelineContext<'a>> for Artifacts {
    async fn apply_tokens(&mut self, context: &'a PipelineContext<'a>) -> Result<()> {
        self.from =
            <PipelineContext as CompleteTokenTransformer>::transform(context, self.from.to_owned())
                .await?;
        self.to =
            <PipelineContext as CompleteTokenTransformer>::transform(context, self.to.to_owned())
                .await?;
        if let Some(after) = self.after.as_mut() {
            self.after = Some(
                <PipelineContext as CompleteTokenTransformer>::transform(context, after.to_owned())
                    .await?,
            );
        }

        Ok(())
    }
}
