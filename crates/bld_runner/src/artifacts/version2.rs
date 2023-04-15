use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
    keywords::version2::{BldDirectory, Environment, RunId, RunStartTime, Variable},
    pipeline::traits::{
        ApplyTokens, DynamicTokenTransformer, HolisticTokenTransformer, StaticTokenTransformer,
    },
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

impl<'a> ApplyTokens<'a, PipelineContext<'a>> for Artifacts {
    fn apply_tokens(&mut self, context: &'a PipelineContext<'a>) -> Result<()>
    where
        Self: Sized,
        PipelineContext<'a>: StaticTokenTransformer<'a, BldDirectory>
            + DynamicTokenTransformer<'a, Variable>
            + DynamicTokenTransformer<'a, Environment>
            + StaticTokenTransformer<'a, RunId>
            + StaticTokenTransformer<'a, RunStartTime>,
    {
        self.from =
            <PipelineContext as HolisticTokenTransformer>::transform(context, self.from.to_owned());
        self.to =
            <PipelineContext as HolisticTokenTransformer>::transform(context, self.to.to_owned());
        self.after = self.after.as_mut().map(|x| {
            <PipelineContext as HolisticTokenTransformer>::transform(context, x.to_owned())
        });
        Ok(())
    }
}
