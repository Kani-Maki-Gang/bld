use std::fmt::Display;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
    pipeline::traits::{ApplyTokens, CompleteTokenTransformer},
    token_context::v2::PipelineContext,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Platform {
    ContainerOrMachine(String),
    Pull {
        image: String,
        pull: bool,
    },
    Build {
        name: String,
        tag: String,
        dockerfile: String,
    },
}

impl Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ContainerOrMachine(image) if image == "machine" => write!(f, "machine"),
            Self::ContainerOrMachine(image) => write!(f, "{image}"),
            Self::Pull { image, .. } => write!(f, "{image}"),
            Self::Build { name, tag, .. } => write!(f, "{name}:{tag}"),
        }
    }
}

#[async_trait]
impl<'a> ApplyTokens<'a, PipelineContext<'a>> for Platform {
    async fn apply_tokens(&mut self, context: &'a PipelineContext) -> Result<()> {
        match self {
            Platform::Pull { image, .. } => {
                *image = context.transform(image.to_owned()).await?;
            }
            Platform::Build {
                name,
                tag,
                dockerfile,
            } => {
                *name = context.transform(name.to_owned()).await?;
                *tag = context.transform(tag.to_owned()).await?;
                *dockerfile = context.transform(dockerfile.to_owned()).await?;
            }
            Platform::ContainerOrMachine(image) if image != "machine" => {
                *image = context.transform(image.to_owned()).await?;
            }
            _ => {}
        }
        Ok(())
    }
}
