use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "all")]
use {
    crate::validator::v3::{Validate, ValidatorContext},
    tracing::debug,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadArtifact {
    #[serde(default = "UploadArtifact::default_id")]
    pub id: String,
    pub upload: String,
    pub name: String,
}

impl UploadArtifact {
    fn default_id() -> String {
        Uuid::new_v4().to_string()
    }
}

impl Default for UploadArtifact {
    fn default() -> Self {
        Self {
            id: Self::default_id(),
            upload: String::new(),
            name: String::new(),
        }
    }
}

#[cfg(feature = "all")]
impl<'a> Validate<'a> for UploadArtifact {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        debug!("Validating upload artifact {}", self.id);

        debug!("Validating artifact's name");
        ctx.push_section("upload");
        ctx.validate_expressions(&self.upload);
        ctx.pop_section();

        debug!("Validating artifact's to");
        ctx.push_section("name");
        if ctx.contains_expressions(&self.name) {
            ctx.append_error("Expressions not supported");
        }
        ctx.pop_section();
    }
}
