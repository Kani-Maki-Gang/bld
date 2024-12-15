use std::{collections::HashSet, sync::Arc};

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
                let inputs: HashSet<&'a str> = pip.inputs.keys().map(|x| x.as_str()).collect();
                let env: HashSet<&'a str> = pip.env.keys().map(|x| x.as_str()).collect();
                let validator = CommonValidator::new(pip, config, fs, inputs, env)?;
                Ok(Self::Pipeline(validator))
            }

            RunnerFile::ActionFileType(action) => {
                let inputs: HashSet<&'a str> = action.inputs.keys().map(|x| x.as_str()).collect();
                let env = HashSet::<&'a str>::new();
                let validator = CommonValidator::new(action, config, fs, inputs, env)?;
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
