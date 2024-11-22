use std::sync::Arc;

use anyhow::Result;
use bld_config::BldConfig;
use bld_core::fs::FileSystem;

use crate::{files::v3::RunnerFile, traits::Validate};

use super::pipeline_v3::PipelineValidator;

pub enum RunnerFileValidator<'a> {
    Pipeline(PipelineValidator<'a>),
    Action,
}

impl<'a> RunnerFileValidator<'a> {
    pub fn new(file: &'a RunnerFile, config: Arc<BldConfig>, fs: Arc<FileSystem>) -> Result<Self> {
        match file {
            RunnerFile::PipelineFileType(pip) => {
                let validator = PipelineValidator::new(pip, config, fs)?;
                Ok(Self::Pipeline(validator))
            }

            RunnerFile::ActionFileType(_) => unimplemented!(),
        }
    }
}

impl<'a> Validate for RunnerFileValidator<'a> {
    async fn validate(self) -> Result<()> {
        match self {
            Self::Pipeline(validator) => validator.validate().await,
            Self::Action => Ok(()),
        }
    }
}
