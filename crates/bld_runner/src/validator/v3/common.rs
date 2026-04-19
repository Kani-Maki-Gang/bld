use std::{collections::HashMap, fmt::Write, path::PathBuf, sync::Arc};

use anyhow::{Result, bail};
use bld_config::{BldConfig, path};
use bld_core::fs::FileSystem;
use bld_pkg::PackageManager;
use regex::Regex;
use tracing::debug;

use crate::expr::v3::{
    context::CommonReadonlyRuntimeExprContext,
    exec::CommonExprExecutor,
    parser::EXPR_REGEX,
    traits::{EvalExpr, EvalObject, WritableRuntimeExprContext},
};

use super::{ConsumeValidator, Validate, ValidatorContext};

pub fn create_expression_regex() -> Result<Regex> {
    Ok(Regex::new(EXPR_REGEX)?)
}

enum Section<'a> {
    Job(&'a str),
    Other(&'a str),
}

impl<'a> Section<'a> {
    pub fn inner(&self) -> &'a str {
        match self {
            Section::Other(s) | Section::Job(s) => s,
        }
    }
}

pub struct ValidatorWritableRuntimeExprContext<'a> {
    exec_id: &'a str,
    outputs: HashMap<String, String>,
}

impl<'a> ValidatorWritableRuntimeExprContext<'a> {
    pub fn new(exec_id: &'a str) -> Self {
        Self {
            exec_id,
            outputs: HashMap::new(),
        }
    }
}

impl<'a> WritableRuntimeExprContext for ValidatorWritableRuntimeExprContext<'a> {
    fn get_exec_id(&self) -> Option<&str> {
        Some(self.exec_id)
    }

    fn get_output<'b>(&'b self, _id: &str, _name: &str) -> Result<&'b str> {
        Ok("")
    }

    fn set_output(&mut self, _id: &str, name: String, value: String) -> Result<()> {
        self.outputs.insert(name, value);
        Ok(())
    }

    fn set_outputs(&mut self, _id: &str, outputs: HashMap<String, String>) -> Result<()> {
        self.outputs = outputs;
        Ok(())
    }
}

pub struct CommonValidator<'a, V: Validate<'a> + EvalObject<'a>> {
    validatable: &'a V,
    config: Arc<BldConfig>,
    file_system: Arc<FileSystem>,
    package_manager: Arc<PackageManager>,
    expr_regex: Regex,
    expr_rctx: &'a CommonReadonlyRuntimeExprContext,
    expr_wctx: &'a [ValidatorWritableRuntimeExprContext<'a>],
    section: Vec<Section<'a>>,
    current_job: Option<Section<'a>>,
    errors: String,
}

impl<'a, V: Validate<'a> + EvalObject<'a>> CommonValidator<'a, V> {
    pub fn new(
        validatable: &'a V,
        config: Arc<BldConfig>,
        file_system: Arc<FileSystem>,
        package_manager: Arc<PackageManager>,
        expr_rctx: &'a CommonReadonlyRuntimeExprContext,
        expr_wctx: &'a [ValidatorWritableRuntimeExprContext],
    ) -> Result<Self> {
        Ok(Self {
            validatable,
            config,
            file_system,
            package_manager,
            expr_regex: create_expression_regex()?,
            expr_rctx,
            expr_wctx,
            section: Vec::new(),
            current_job: None,
            errors: String::new(),
        })
    }

    fn section_txt(&self) -> String {
        self.section
            .iter()
            .map(|x| x.inner())
            .collect::<Vec<&'a str>>()
            .join(" > ")
    }
}

impl<'a, V: Validate<'a> + EvalObject<'a>> ValidatorContext<'a> for CommonValidator<'a, V> {
    fn get_config(&self) -> Arc<BldConfig> {
        self.config.clone()
    }

    fn get_fs(&self) -> Arc<FileSystem> {
        self.file_system.clone()
    }

    fn get_package_manager(&self) -> Arc<PackageManager> {
        self.package_manager.clone()
    }

    fn push_section(&mut self, section: &'a str) {
        self.section.push(Section::Other(section));
    }

    fn push_job_section(&mut self, section: &'a str) {
        self.section.push(Section::Job(section));
        self.current_job = Some(Section::Job(section));
    }

    fn pop_section(&mut self) {
        let section = self.section.pop();
        if matches!(section, Some(Section::Job(_))) {
            self.current_job = None;
        }
    }

    fn clear_section(&mut self) {
        self.section.clear();
    }

    fn append_error(&mut self, error: &str) {
        let section = self.section_txt();
        let _ = writeln!(self.errors, "[{section}] {error}");
    }

    fn contains_expressions(&mut self, value: &str) -> bool {
        self.expr_regex.find(value).is_some()
    }

    fn validate_expressions(&mut self, value: &'a str) {
        let expr_wctx = self
            .expr_wctx
            .iter()
            .find(|x| x.get_exec_id() == self.current_job.as_ref().map(|x| x.inner()))
            .or_else(|| self.expr_wctx.iter().next());

        let Some(expr_wctx) = expr_wctx else { return };

        let expr_exec = CommonExprExecutor::new(self.validatable, self.expr_rctx, expr_wctx);
        for entry in self.expr_regex.find_iter(value) {
            let Err(e) = expr_exec.eval(entry.as_str()) else {
                continue;
            };
            let section = self.section_txt();
            let _ = writeln!(self.errors, "[{section}] {}", e);
        }
    }

    fn validate_file_path(&mut self, value: &'a str) {
        if self.contains_expressions(value) {
            return;
        }
        let path = path![value];
        if !path.is_file() {
            let section = self.section_txt();
            let _ = writeln!(self.errors, "[{section} > {value}] File not found");
        }
    }

    fn validate_env(&mut self, env: &'a HashMap<String, String>) {
        for (k, v) in env.iter() {
            debug!("Validating env: {}", k);
            self.section.push(Section::Other(k));
            self.validate_expressions(v);
            self.section.pop();
        }
    }
}

impl<'a, V: Validate<'a> + EvalObject<'a>> ConsumeValidator for CommonValidator<'a, V> {
    async fn validate(mut self) -> Result<()> {
        self.validatable.validate(&mut self).await;
        if self.errors.is_empty() {
            Ok(())
        } else {
            bail!(self.errors)
        }
    }
}
