use std::sync::Arc;

use anyhow::Result;
use bld_config::BldConfig;
use bld_core::fs::FileSystem;
use bld_pkg::PackageManager;
use bld_utils::sync::IntoArc;

use crate::{
    expr::v3::context::{CommonReadonlyRuntimeExprContext, CommonWritableRuntimeExprContext},
    files::v3::RunnerFile,
};

use super::{CommonValidator, ConsumeValidator};

pub struct RunnerFileValidator<'a> {
    file: &'a RunnerFile,
    config: Arc<BldConfig>,
    file_system: Arc<FileSystem>,
    package_manager: Arc<PackageManager>,
    expr_rctx: CommonReadonlyRuntimeExprContext,
    expr_wctx: Vec<CommonWritableRuntimeExprContext<'a>>,
}

impl<'a> RunnerFileValidator<'a> {
    pub fn new(
        file: &'a RunnerFile,
        config: Arc<BldConfig>,
        file_system: Arc<FileSystem>,
        package_manager: Arc<PackageManager>,
    ) -> Result<Self> {
        let expr_rctx = CommonReadonlyRuntimeExprContext::new(
            config.clone(),
            file.inputs_map().into_arc(),
            file.env_map().into_arc(),
            String::new(),
            String::new(),
        );
        let expr_wctx = match &file {
            RunnerFile::PipelineFileType(pipeline) => pipeline
                .jobs
                .keys()
                .map(|k| CommonWritableRuntimeExprContext::new(k))
                .collect(),
            RunnerFile::ActionFileType(_) => {
                vec![CommonWritableRuntimeExprContext::new("action")]
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
