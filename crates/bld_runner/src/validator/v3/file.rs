use std::sync::Arc;

use anyhow::Result;
use bld_config::BldConfig;
use bld_core::fs::FileSystem;

use crate::{action::v3::Action, files::v3::RunnerFile, pipeline::v3::Pipeline};

use super::{CommonValidator, ConsumeValidator};

pub enum RunnerFileValidator<'a> {
    Pipeline(CommonValidator<'a, Pipeline>),
    Action(CommonValidator<'a, Action>),
}

impl<'a> RunnerFileValidator<'a> {
    pub fn new(file: &'a RunnerFile, config: Arc<BldConfig>, fs: Arc<FileSystem>) -> Result<Self> {
        match file {
            RunnerFile::PipelineFileType(pip) => {
                let validator = CommonValidator::new(pip, config, fs, &pip.inputs, &pip.env)?;
                Ok(Self::Pipeline(validator))
            }

            RunnerFile::ActionFileType(action) => {
                let validator =
                    CommonValidator::new(action, config, fs, &action.inputs, &action.env)?;
                Ok(Self::Action(validator))
            }
        }
    }
}

impl<'a> ConsumeValidator for RunnerFileValidator<'a> {
    async fn validate(self) -> Result<()> {
        match self {
            Self::Pipeline(validator) => validator.validate().await,
            Self::Action(validator) => validator.validate().await,
        }
    }
}
