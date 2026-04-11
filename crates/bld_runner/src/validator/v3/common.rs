use std::{collections::HashMap, fmt::Write, path::PathBuf, sync::Arc};

use anyhow::{Result, bail};
use bld_config::{BldConfig, path};
use bld_core::fs::FileSystem;
use bld_pkg::PackageManager;
use regex::Regex;
use tracing::debug;

use crate::expr::v3::{
    context::{CommonReadonlyRuntimeExprContext, CommonWritableRuntimeExprContext},
    exec::CommonExprExecutor,
    parser::EXPR_REGEX,
    traits::{EvalExpr, EvalObject},
};

use super::{ConsumeValidator, Validate, ValidatorContext};

pub fn create_expression_regex() -> Result<Regex> {
    Ok(Regex::new(EXPR_REGEX)?)
}

pub struct CommonValidator<'a, V: Validate<'a> + EvalObject<'a>> {
    validatable: &'a V,
    config: Arc<BldConfig>,
    file_system: Arc<FileSystem>,
    package_manager: Arc<PackageManager>,
    expr_regex: Regex,
    expr_rctx: &'a CommonReadonlyRuntimeExprContext,
    expr_wctx: &'a CommonWritableRuntimeExprContext,
    section: Vec<&'a str>,
    errors: String,
}

impl<'a, V: Validate<'a> + EvalObject<'a>> CommonValidator<'a, V> {
    pub fn new(
        validatable: &'a V,
        config: Arc<BldConfig>,
        file_system: Arc<FileSystem>,
        package_manager: Arc<PackageManager>,
        expr_rctx: &'a CommonReadonlyRuntimeExprContext,
        expr_wctx: &'a CommonWritableRuntimeExprContext,
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
            errors: String::new(),
        })
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
        self.section.push(section);
    }

    fn pop_section(&mut self) {
        self.section.pop();
    }

    fn clear_section(&mut self) {
        self.section.clear();
    }

    fn append_error(&mut self, error: &str) {
        let section = self.section.join(" > ");
        let _ = writeln!(self.errors, "[{section}] {error}");
    }

    fn contains_symbols(&mut self, value: &str) -> bool {
        self.expr_regex.find(value).is_some()
    }

    fn validate_symbols(&mut self, value: &'a str) {
        let expr_exec = CommonExprExecutor::new(self.validatable, self.expr_rctx, self.expr_wctx);
        for entry in self.expr_regex.find_iter(value) {
            let Err(e) = expr_exec.eval(entry.as_str()) else {
                continue;
            };
            let section = self.section.join(" > ");
            let _ = writeln!(self.errors, "[{section}] {}", e);
        }
    }

    fn validate_file_path(&mut self, value: &'a str) {
        if self.contains_symbols(value) {
            return;
        }
        let path = path![value];
        if !path.is_file() {
            let section = self.section.join(" > ");
            let _ = writeln!(self.errors, "[{section} > {value}] File not found");
        }
    }

    fn validate_env(&mut self, env: &'a HashMap<String, String>) {
        for (k, v) in env.iter() {
            debug!("Validating env: {}", k);
            self.section.push(k);
            self.validate_symbols(v);
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
