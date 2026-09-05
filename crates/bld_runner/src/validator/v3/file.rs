use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::Result;
use bld_config::BldConfig;
use bld_core::fs::FileSystem;
use bld_pkg::PackageManager;
use bld_utils::sync::IntoArc;

use crate::{
    expr::v3::context::CommonReadonlyRuntimeExprContext, files::v3::RunnerFile,
    validator::v3::ValidatorWritableRuntimeExprContext,
};

use super::{CommonValidator, ConsumeValidator};

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
        let with_blank_values = |keys: Vec<&String>| -> HashMap<String, String> {
            keys.into_iter()
                .map(|k| (k.clone(), String::new()))
                .collect()
        };
        match self.file {
            RunnerFile::PipelineFileType(pip) => {
                let inputs = with_blank_values(pip.inputs.keys().collect()).into_arc();
                let env = with_blank_values(pip.env.keys().collect());
                let expr_rctx = CommonReadonlyRuntimeExprContext::new(
                    self.config.clone(),
                    inputs.clone(),
                    env.clone().into_arc(),
                    String::new(),
                    String::new(),
                )
                .with_validation();
                // Create expr_rctx for each job's scope
                let job_expr_rctx_values: Vec<(&str, CommonReadonlyRuntimeExprContext)> = pip
                    .jobs
                    .iter()
                    .filter(|(_, job)| !job.env.is_empty())
                    .map(|(name, job)| {
                        let mut job_env = env.clone();
                        job_env.extend(with_blank_values(job.env.keys().collect()));
                        (
                            name.as_str(),
                            CommonReadonlyRuntimeExprContext::new(
                                self.config.clone(),
                                inputs.clone(),
                                job_env.into_arc(),
                                String::new(),
                                String::new(),
                            )
                            .with_validation(),
                        )
                    })
                    .collect();
                let job_expr_rctx: HashMap<&str, &CommonReadonlyRuntimeExprContext> =
                    job_expr_rctx_values
                        .iter()
                        .map(|(name, rctx)| (*name, rctx))
                        .collect();
                let expr_wctx: Vec<ValidatorWritableRuntimeExprContext<'_>> = pip
                    .jobs
                    .keys()
                    .map(|k| ValidatorWritableRuntimeExprContext::new(k.as_str()))
                    .collect();
                let job_needs: HashMap<&str, HashSet<&str>> = pip
                    .jobs
                    .iter()
                    .map(|(name, job)| (name.as_str(), job.needs_iter().collect()))
                    .collect();
                CommonValidator::new(
                    pip.as_ref(),
                    self.config,
                    self.file_system,
                    self.package_manager,
                    &expr_rctx,
                    &expr_wctx,
                )?
                .with_job_needs(job_needs)
                .with_job_expr_rctx(job_expr_rctx)
                .validate()
                .await
            }
            RunnerFile::ActionFileType(action) => {
                let expr_rctx = CommonReadonlyRuntimeExprContext::new(
                    self.config.clone(),
                    with_blank_values(action.inputs.keys().collect()).into_arc(),
                    HashMap::new().into_arc(),
                    String::new(),
                    String::new(),
                )
                .with_validation();
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
