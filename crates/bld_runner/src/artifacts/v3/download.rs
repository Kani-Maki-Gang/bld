use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "all")]
use {
    crate::validator::v3::{Validate, ValidatorContext},
    tracing::debug,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DownloadArtifact {
    #[serde(default = "DownloadArtifact::default_id")]
    pub id: String,
    pub download: String,
    pub to: String,
}

impl DownloadArtifact {
    fn default_id() -> String {
        Uuid::new_v4().to_string()
    }
}

impl Default for DownloadArtifact {
    fn default() -> Self {
        Self {
            id: Self::default_id(),
            download: String::new(),
            to: String::new(),
        }
    }
}

#[cfg(feature = "all")]
impl<'a> Validate<'a> for DownloadArtifact {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        debug!("Validating download artifact {}", self.id);

        debug!("Validating artifact's download field");
        ctx.push_section("download");
        if ctx.contains_expressions(&self.download) {
            ctx.append_error("Expressions not supported");
        }
        ctx.pop_section();

        debug!("Validating artifact's to field");
        ctx.push_section("to");
        ctx.validate_expressions(&self.to);
        ctx.pop_section();
    }
}
