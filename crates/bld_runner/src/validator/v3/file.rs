use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use bld_config::BldConfig;
use bld_core::fs::FileSystem;
use bld_pkg::PackageManager;
use bld_utils::sync::IntoArc;

use crate::{
    expr::v3::context::{CommonReadonlyRuntimeExprContextOptions, expr_rctx},
    files::v3::RunnerFile,
    validator::v3::ValidatorWritableRuntimeExprContext,
};

use super::{CommonValidator, ConsumeValidator, EMPTY_ENV, input_placeholders};

pub struct RunnerFileValidator<'a> {
    file: &'a RunnerFile,
    config: Arc<BldConfig>,
    file_system: Arc<FileSystem>,
    package_manager: Arc<PackageManager>,
}

impl<'a> RunnerFileValidator<'a> {
    pub fn new(
        file: &'a RunnerFile,
        config: Arc<BldConfig>,
        file_system: Arc<FileSystem>,
        package_manager: Arc<PackageManager>,
    ) -> Self {
        Self {
            file,
            config,
            file_system,
            package_manager,
        }
    }
}

impl ConsumeValidator for RunnerFileValidator<'_> {
    async fn validate(self) -> Result<()> {
        match self.file {
            RunnerFile::PipelineFileType(pip) => {
                let expr_rctx = expr_rctx(CommonReadonlyRuntimeExprContextOptions {
                    obj: pip.as_ref(),
                    config: self.config.clone(),
                    inputs: input_placeholders(&pip.inputs).into_arc(),
                    declared_inputs: &pip.inputs,
                    env: HashMap::new().into_arc(),
                    declared_env: &pip.env,
                    run_id: String::new(),
                    run_start_time: String::new(),
                })?;
                let expr_wctx: Vec<ValidatorWritableRuntimeExprContext<'_>> = pip
                    .jobs
                    .keys()
                    .map(|k| ValidatorWritableRuntimeExprContext::new(k.as_str()))
                    .collect();
                CommonValidator::new(
                    pip.as_ref(),
                    self.config,
                    self.file_system,
                    self.package_manager,
                    &expr_rctx,
                    &expr_wctx,
                )?
                .validate()
                .await
            }
            RunnerFile::ActionFileType(action) => {
                let expr_rctx = expr_rctx(CommonReadonlyRuntimeExprContextOptions {
                    obj: action.as_ref(),
                    config: self.config.clone(),
                    inputs: input_placeholders(&action.inputs).into_arc(),
                    declared_inputs: &action.inputs,
                    env: HashMap::new().into_arc(),
                    declared_env: &EMPTY_ENV,
                    run_id: String::new(),
                    run_start_time: String::new(),
                })?;
                let expr_wctx = vec![ValidatorWritableRuntimeExprContext::new("action")];
                CommonValidator::new(
                    action.as_ref(),
                    self.config,
                    self.file_system,
                    self.package_manager,
                    &expr_rctx,
                    &expr_wctx,
                )?
                .validate()
                .await
            }
        }
    }
}
