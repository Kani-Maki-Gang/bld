use std::fmt::Display;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
    pipeline::traits::{ApplyTokens, CompleteTokenTransformer},
    token_context::version2::PipelineContext,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Platform {
    ContainerOrMachine(String),
    ContainerByPull {
        image: String,
        pull: bool,
    },
    ContainerByBuild {
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
            Self::ContainerByPull { image, .. } => write!(f, "{image}"),
            Self::ContainerByBuild { name, tag, .. } => write!(f, "{name}:{tag}"),
        }
    }
}

#[async_trait]
impl<'a> ApplyTokens<'a, PipelineContext<'a>> for Platform {
    async fn apply_tokens(&mut self, context: &'a PipelineContext) -> Result<()> {
        match self {
            Platform::ContainerByPull { image, .. } => {
                *image = <PipelineContext as CompleteTokenTransformer>::transform(
                    context,
                    image.to_owned(),
                )
                .await?;
            }
            Platform::ContainerByBuild {
                name,
                tag,
                dockerfile,
            } => {
                *name = <PipelineContext as CompleteTokenTransformer>::transform(
                    context,
                    name.to_owned(),
                )
                .await?;
                *tag = <PipelineContext as CompleteTokenTransformer>::transform(
                    context,
                    tag.to_owned(),
                )
                .await?;
                *dockerfile = <PipelineContext as CompleteTokenTransformer>::transform(
                    context,
                    dockerfile.to_owned(),
                )
                .await?;
            }
            Platform::ContainerOrMachine(image) if image != "machine" => {
                *image = <PipelineContext as CompleteTokenTransformer>::transform(
                    context,
                    image.to_owned(),
                )
                .await?;
            }
            _ => {}
        }
        Ok(())
    }
}
