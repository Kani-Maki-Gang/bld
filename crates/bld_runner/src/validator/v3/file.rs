use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use bld_config::BldConfig;
use bld_core::fs::FileSystem;
use bld_pkg::PackageManager;
use bld_utils::sync::IntoArc;

use crate::{
    expr::v3::context::{
        CommonReadonlyRuntimeExprContext, CommonReadonlyRuntimeExprContextOptions, expr_rctx,
    },
    files::v3::RunnerFile,
    validator::v3::ValidatorWritableRuntimeExprContext,
};

use super::{CommonValidator, ConsumeValidator, EMPTY_ENV, input_placeholders};

pub struct RunnerFileValidator<'a> {
    file: &'a RunnerFile,
    config: Arc<BldConfig>,
    file_system: Arc<FileSystem>,
    package_manager: Arc<PackageManager>,
    expr_rctx: CommonReadonlyRuntimeExprContext,
    expr_wctx: Vec<ValidatorWritableRuntimeExprContext<'a>>,
}

impl<'a> RunnerFileValidator<'a> {
    pub fn new(
        file: &'a RunnerFile,
        config: Arc<BldConfig>,
        file_system: Arc<FileSystem>,
        package_manager: Arc<PackageManager>,
    ) -> Result<Self> {
        let expr_rctx = Self::expr_rctx(file, config.clone());
        let expr_wctx = match &file {
            RunnerFile::PipelineFileType(pipeline) => pipeline
                .jobs
                .keys()
                .map(|k| ValidatorWritableRuntimeExprContext::new(k))
                .collect(),
            RunnerFile::ActionFileType(_) => {
                vec![ValidatorWritableRuntimeExprContext::new("action")]
            }
        };
        Ok(Self {
            file,
            config,
            file_system,
            package_manager,
            expr_rctx,
            expr_wctx,
        })
    }

    fn expr_rctx(file: &RunnerFile, config: Arc<BldConfig>) -> CommonReadonlyRuntimeExprContext {
        let resolved = match file {
            RunnerFile::PipelineFileType(pipeline) => {
                expr_rctx(CommonReadonlyRuntimeExprContextOptions {
                    obj: pipeline.as_ref(),
                    config: config.clone(),
                    inputs: input_placeholders(&pipeline.inputs).into_arc(),
                    declared_inputs: &pipeline.inputs,
                    env: HashMap::new().into_arc(),
                    declared_env: &pipeline.env,
                    run_id: String::new(),
                    run_start_time: String::new(),
                })
            }
            RunnerFile::ActionFileType(action) => {
                expr_rctx(CommonReadonlyRuntimeExprContextOptions {
                    obj: action.as_ref(),
                    config: config.clone(),
                    inputs: input_placeholders(&action.inputs).into_arc(),
                    declared_inputs: &action.inputs,
                    env: HashMap::new().into_arc(),
                    declared_env: &EMPTY_ENV,
                    run_id: String::new(),
                    run_start_time: String::new(),
                })
            }
        };

        resolved.unwrap_or_else(|_| {
            CommonReadonlyRuntimeExprContext::new(
                config,
                file.inputs_map().into_arc(),
                file.env_map().into_arc(),
                String::new(),
                String::new(),
            )
        })
    }
}

impl ConsumeValidator for RunnerFileValidator<'_> {
    async fn validate(self) -> Result<()> {
        match self.file {
            RunnerFile::PipelineFileType(pip) => {
                CommonValidator::new(
                    pip.as_ref(),
                    self.config,
                    self.file_system,
                    self.package_manager,
                    &self.expr_rctx,
                    &self.expr_wctx,
                )?
                .validate()
                .await
            }
            RunnerFile::ActionFileType(action) => {
                CommonValidator::new(
                    action.as_ref(),
                    self.config,
                    self.file_system,
                    self.package_manager,
                    &self.expr_rctx,
                    &self.expr_wctx,
                )?
                .validate()
                .await
            }
        }
    }
}
