use serde::{Deserialize, Serialize};

#[cfg(feature = "all")]
use tracing::debug;

#[cfg(feature = "all")]
use crate::validator::v3::{Validate, ValidatorContext};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Artifacts {
    pub method: String,
    pub from: String,
    pub to: String,
    pub ignore_errors: Option<bool>,
    pub after: Option<String>,
}

#[cfg(feature = "all")]
impl<'a> Validate<'a> for Artifacts {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        debug!("Validating artifact's from section");
        ctx.push_section("from");
        ctx.validate_symbols(&self.from);
        ctx.pop_section();

        debug!("Validating artifact's to section");
        ctx.push_section("to");
        ctx.validate_symbols(&self.to);
        ctx.pop_section();

        unimplemented!();
        // TODO: Implement the following:
        // self.validate_artifact_after(artifact.after.as_ref());
    }
}
