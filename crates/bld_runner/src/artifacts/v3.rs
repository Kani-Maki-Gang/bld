use serde::{Deserialize, Serialize};

#[cfg(feature = "all")]
use anyhow::Result;

#[cfg(feature = "all")]
use crate::token_context::v3::ExecutionContext;
use crate::validator::v3::{Validatable, ValidatorContext};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Artifacts {
    pub method: String,
    pub from: String,
    pub to: String,
    pub ignore_errors: Option<bool>,
    pub after: Option<String>,
}

impl Artifacts {
    #[cfg(feature = "all")]
    pub async fn apply_tokens<'a>(&mut self, context: &'a ExecutionContext<'a>) -> Result<()> {
        self.from = context.transform(self.from.to_owned()).await?;
        self.to = context.transform(self.to.to_owned()).await?;

        if let Some(after) = self.after.as_mut() {
            self.after = Some(context.transform(after.to_owned()).await?);
        }

        Ok(())
    }
}

impl<'a> Validatable<'a> for Artifacts {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        ctx.push_section("from");
        ctx.validate_symbols(&self.from);
        ctx.pop_section();

        ctx.push_section("to");
        ctx.validate_symbols(&self.to);
        ctx.pop_section();

        unimplemented!();
        // TODO: Implement the following:
        // self.validate_artifact_after(artifact.after.as_ref());
    }
}
