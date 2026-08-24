use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    path::PathBuf,
    sync::Arc,
};

use anyhow::{Result, bail};
use bld_config::{BldConfig, path};
use bld_core::fs::FileSystem;
use bld_pkg::PackageManager;
use regex::Regex;
use tracing::debug;

use crate::expr::v3::{
    context::{CommonReadonlyRuntimeExprContext, START_OF_RUN_WCTX},
    exec::CommonExprExecutor,
    parser,
    parser::{ExprParser, Rule},
    traits::{EvalExpr, EvalObject, ExprValue, OutputScope, WritableRuntimeExprContext},
};
use pest::{Parser, iterators::Pairs};

use super::{ConsumeValidator, ExprScope, Validate, ValidatorContext};

fn collect_matrix_refs(pairs: Pairs<Rule>, result: &mut Vec<String>) {
    for pair in pairs {
        if pair.as_rule() == Rule::Object {
            let mut parts = pair.into_inner();
            let Some(first) = parts.next() else {
                continue;
            };
            if first.as_str() != "matrix" {
                continue;
            }
            if let Some(second) = parts.next()
                && second.as_rule() == Rule::ObjectPart
            {
                result.push(second.as_str().to_string());
            }
        } else {
            collect_matrix_refs(pair.into_inner(), result);
        }
    }
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

    fn get_output<'b>(
        &'b self,
        _scope: OutputScope,
        _id: &str,
        _name: &str,
    ) -> Result<ExprValue<'b>> {
        Ok(ExprValue::Unknown)
    }

    fn set_output(&mut self, _id: &str, name: String, value: String) -> Result<()> {
        self.outputs.insert(name, value);
        Ok(())
    }

    fn set_outputs(&mut self, _id: &str, outputs: HashMap<String, String>) -> Result<()> {
        self.outputs = outputs;
        Ok(())
    }

    fn get_matrix_value<'b>(&'b self, _name: &str) -> Result<&'b str> {
        Ok("")
    }
}

pub struct CommonValidator<'a, V: Validate<'a> + for<'x> EvalObject<'x>> {
    validatable: &'a V,
    config: Arc<BldConfig>,
    file_system: Arc<FileSystem>,
    package_manager: Arc<PackageManager>,
    expr_regex: Regex,
    expr_rctx: &'a CommonReadonlyRuntimeExprContext,
    job_expr_rctx: HashMap<&'a str, &'a CommonReadonlyRuntimeExprContext>,
    expr_wctx: &'a [ValidatorWritableRuntimeExprContext<'a>],
    job_needs: HashMap<&'a str, HashSet<&'a str>>,
    section: Vec<Section<'a>>,
    current_job: Option<Section<'a>>,
    errors: String,
}

impl<'a, V: Validate<'a> + for<'x> EvalObject<'x>> CommonValidator<'a, V> {
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
            expr_regex: parser::new_regex()?,
            expr_rctx,
            job_expr_rctx: HashMap::new(),
            expr_wctx,
            job_needs: HashMap::new(),
            section: Vec::new(),
            current_job: None,
            errors: String::new(),
        })
    }

    pub fn with_job_needs(mut self, job_needs: HashMap<&'a str, HashSet<&'a str>>) -> Self {
        self.job_needs = job_needs;
        self
    }

    pub fn with_job_expr_rctx(
        mut self,
        job_expr_rctx: HashMap<&'a str, &'a CommonReadonlyRuntimeExprContext>,
    ) -> Self {
        self.job_expr_rctx = job_expr_rctx;
        self
    }

    fn rctx(&self) -> &'a CommonReadonlyRuntimeExprContext {
        self.current_job
            .as_ref()
            .and_then(|job| self.job_expr_rctx.get(job.inner()))
            .copied()
            .unwrap_or(self.expr_rctx)
    }

    fn validate_job_output_refs(&mut self, value: &'a str) {
        let Some(current_job) = self.current_job.as_ref().map(|x| x.inner()) else {
            return;
        };
        let needs = self.job_needs.get(current_job);
        for job_name in self.job_output_refs(value) {
            let allowed = needs
                .map(|n| n.contains(job_name.as_str()))
                .unwrap_or(false);
            if !allowed {
                let section = self.section_txt();
                let _ = writeln!(
                    self.errors,
                    "[{section}] job '{job_name}' is not defined in the needs of job '{current_job}', only jobs listed in needs can have their outputs read"
                );
            }
        }
    }

    fn section_txt(&self) -> String {
        self.section
            .iter()
            .map(|x| x.inner())
            .collect::<Vec<&'a str>>()
            .join(" > ")
    }

    fn runtime_wctx(&self) -> Option<&'a ValidatorWritableRuntimeExprContext<'a>> {
        self.expr_wctx
            .iter()
            .find(|x| x.get_exec_id() == self.current_job.as_ref().map(|x| x.inner()))
            .or_else(|| self.expr_wctx.iter().next())
    }

    fn eval_expressions<W: WritableRuntimeExprContext>(&mut self, value: &'a str, wctx: &'a W) {
        let expr_exec = CommonExprExecutor::new(self.validatable, self.rctx(), wctx);
        for entry in self.expr_regex.find_iter(value) {
            let Err(e) = expr_exec.eval(entry.as_str()) else {
                continue;
            };
            let section = self.section_txt();
            let _ = writeln!(self.errors, "[{section}] {}", e);
        }
    }

    fn eval_array_expression<W: WritableRuntimeExprContext>(
        &mut self,
        value: &'a str,
        wctx: &'a W,
    ) {
        let expr_exec = CommonExprExecutor::new(self.validatable, self.rctx(), wctx);
        for entry in self.expr_regex.find_iter(value) {
            match expr_exec.eval(entry.as_str()) {
                // The value of a step output isn't known during validation, so an
                // expression that uses one can't be checked against the array type.
                Ok(ExprValue::Array(_) | ExprValue::Unknown) => {}
                Ok(other) => {
                    let section = self.section_txt();
                    let _ = writeln!(
                        self.errors,
                        "[{section}] expected an array, found {}",
                        other.type_as_string()
                    );
                }
                Err(e) => {
                    let section = self.section_txt();
                    let _ = writeln!(self.errors, "[{section}] {}", e);
                }
            }
        }
    }

    fn eval_condition_expression<W: WritableRuntimeExprContext>(
        &mut self,
        value: &'a str,
        wctx: &'a W,
    ) {
        let expr_exec = CommonExprExecutor::new(self.validatable, self.rctx(), wctx);
        for entry in self.expr_regex.find_iter(value) {
            match expr_exec.eval(entry.as_str()) {
                Ok(ExprValue::Boolean(_)) | Ok(ExprValue::Text(_)) | Ok(ExprValue::Unknown) => {}
                Ok(value) => {
                    let section = self.section_txt();
                    let _ = writeln!(
                        self.errors,
                        "[{section}] a condition must give a boolean value, but it gives {}",
                        value.type_as_string()
                    );
                }
                Err(e) => {
                    let section = self.section_txt();
                    let _ = writeln!(self.errors, "[{section}] {}", e);
                }
            }
        }
    }
}

impl<'a, V: Validate<'a> + for<'x> EvalObject<'x>> ValidatorContext<'a> for CommonValidator<'a, V> {
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
        if section.is_empty() {
            let _ = writeln!(self.errors, "{error}");
        } else {
            let _ = writeln!(self.errors, "[{section}] {error}");
        }
    }

    fn expression_count(&self, value: &str) -> usize {
        self.expr_regex.find_iter(value).count()
    }

    fn contains_expressions(&mut self, value: &str) -> bool {
        self.expr_regex.find(value).is_some()
    }

    fn validate_expressions(&mut self, value: &'a str, scope: ExprScope) {
        match scope {
            ExprScope::StartOfRun => self.eval_expressions(value, &START_OF_RUN_WCTX),
            ExprScope::Runtime => {
                self.validate_job_output_refs(value);
                let Some(expr_wctx) = self.runtime_wctx() else {
                    return;
                };
                self.eval_expressions(value, expr_wctx)
            }
        }
    }

    fn validate_array_expression(&mut self, value: &'a str, scope: ExprScope) {
        match scope {
            ExprScope::StartOfRun => self.eval_array_expression(value, &START_OF_RUN_WCTX),
            ExprScope::Runtime => {
                self.validate_job_output_refs(value);
                let Some(expr_wctx) = self.runtime_wctx() else {
                    return;
                };
                self.eval_array_expression(value, expr_wctx)
            }
        }
    }

    fn matrix_refs(&self, value: &str) -> Vec<String> {
        let mut result = Vec::new();
        for entry in self.expr_regex.find_iter(value) {
            let Ok(pairs) = ExprParser::parse(Rule::Full, entry.as_str()) else {
                continue;
            };
            collect_matrix_refs(pairs, &mut result);
        }
        result
    }

    fn job_output_refs(&self, value: &str) -> Vec<String> {
        let mut result = Vec::new();
        for entry in self.expr_regex.find_iter(value) {
            let mut rest = entry.as_str();
            while let Some(pos) = rest.find("jobs.") {
                let after = &rest[pos + "jobs.".len()..];
                // A job name is a part of an object path, so it keeps every character
                // that the grammar allows in one, hyphens included.
                let end = after
                    .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-'))
                    .unwrap_or(after.len());
                let name = &after[..end];
                let remainder = &after[end..];
                if !name.is_empty() && remainder.starts_with(".outputs.") {
                    result.push(name.to_string());
                }
                rest = &after[end..];
            }
        }
        result
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

    fn validate_env(&mut self, env: &'a HashMap<String, String>, scope: ExprScope) {
        // A declared env section is resolved before the values of that section exist, so
        // its own values are not available to it. The env of a job is validated with the
        // context of the file for that reason, while the env of a call to another file is
        // resolved when the step runs and keeps the context of the job.
        let current_job = match scope {
            ExprScope::StartOfRun => self.current_job.take(),
            ExprScope::Runtime => None,
        };

        for (k, v) in env.iter() {
            debug!("Validating env: {}", k);
            if k.is_empty() {
                self.append_error("Environment variable name must not be empty");
                continue;
            }
            self.section.push(Section::Other(k));
            self.validate_expressions(v, scope);
            self.section.pop();
        }

        if current_job.is_some() {
            self.current_job = current_job;
        }
    }

    fn validate_condition(&mut self, condition: &'a str, scope: ExprScope) {
        let expr_count = self.expression_count(condition);
        if expr_count == 0 {
            self.append_error("Condition must contain exactly one expression");
        } else if expr_count > 1 {
            self.append_error("Condition must contain at most one expression");
        } else {
            match scope {
                ExprScope::StartOfRun => {
                    self.eval_condition_expression(condition, &START_OF_RUN_WCTX)
                }
                ExprScope::Runtime => {
                    let Some(expr_wctx) = self.runtime_wctx() else {
                        return;
                    };
                    self.eval_condition_expression(condition, expr_wctx)
                }
            }
        }
    }
}

impl<'a, V: Validate<'a> + for<'x> EvalObject<'x>> ConsumeValidator for CommonValidator<'a, V> {
    async fn validate(mut self) -> Result<()> {
        self.validatable.validate(&mut self).await;
        if self.errors.is_empty() {
            Ok(())
        } else {
            bail!(self.errors)
        }
    }
}
