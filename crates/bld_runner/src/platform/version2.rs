use std::fmt::Display;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
    pipeline::traits::{
        ApplyTokens, DynamicTokenTransformer, HolisticTokenTransformer, StaticTokenTransformer,
    },
    token_context::version2::PipelineContext,
};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Platform {
    #[default]
    #[serde(rename(serialize = "machine", deserialize = "machine"))]
    Machine,
    Container(String),
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
            Self::Machine => write!(f, "machine"),
            Self::Container(image) => write!(f, "{image}"),
            Self::ContainerByPull { image, .. } => write!(f, "{image}"),
            Self::ContainerByBuild { name, tag, .. } => write!(f, "{name}:{tag}"),
        }
    }
}

impl<'a> ApplyTokens<'a, PipelineContext<'a>> for Platform {
    fn apply_tokens(&mut self, context: &'a PipelineContext) -> Result<()>
    where
        Self: Sized,
        PipelineContext<'a>: StaticTokenTransformer<'a, crate::keywords::version2::BldDirectory>
            + DynamicTokenTransformer<'a, crate::keywords::version2::Variable>
            + DynamicTokenTransformer<'a, crate::keywords::version2::Environment>
            + StaticTokenTransformer<'a, crate::keywords::version2::RunId>
            + StaticTokenTransformer<'a, crate::keywords::version2::RunStartTime>,
    {
        match self {
            Platform::ContainerByPull { image, .. } => {
                *image = <PipelineContext as HolisticTokenTransformer>::transform(
                    context,
                    image.to_owned(),
                );
            }
            Platform::ContainerByBuild {
                name,
                tag,
                dockerfile,
            } => {
                *name = <PipelineContext as HolisticTokenTransformer>::transform(
                    context,
                    name.to_owned(),
                );
                *tag = <PipelineContext as HolisticTokenTransformer>::transform(
                    context,
                    tag.to_owned(),
                );
                *dockerfile = <PipelineContext as HolisticTokenTransformer>::transform(
                    context,
                    dockerfile.to_owned(),
                );
            }
            Platform::Container(image) => {
                *image = <PipelineContext as HolisticTokenTransformer>::transform(
                    context,
                    image.to_owned(),
                );
            }
            _ => {}
        }
        Ok(())
    }
}
