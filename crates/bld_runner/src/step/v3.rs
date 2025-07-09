use std::iter::Peekable;

use crate::{
    expr::v3::{
        parser::Rule,
        traits::{
            EvalObject, ExprText, ExprValue, ReadonlyRuntimeExprContext, WritableRuntimeExprContext,
        },
    },
    external::v3::External,
};
use anyhow::{Result, bail};
use pest::iterators::Pairs;
use serde::{Deserialize, Serialize};

#[cfg(feature = "all")]
use bld_config::BldConfig;

#[cfg(feature = "all")]
use bld_utils::fs::IsYaml;

#[cfg(feature = "all")]
use tracing::debug;

#[cfg(feature = "all")]
use crate::{
    traits::Dependencies,
    validator::v3::{Validate, ValidatorContext},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellCommand {
    pub id: Option<String>,
    pub name: Option<String>,
    pub working_dir: Option<String>,
    pub run: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Step {
    SingleSh(String),
    ComplexSh(Box<ShellCommand>),
    ExternalFile(Box<External>),
}

impl Step {
    pub fn is(&self, id: &str) -> bool {
        let Self::ComplexSh(complex) = self else {
            return false;
        };
        complex.id.as_ref().map(|x| x == id).unwrap_or_default()
    }
}

#[cfg(feature = "all")]
impl Dependencies for Step {
    fn local_deps(&self, config: &BldConfig) -> Vec<String> {
        match self {
            Self::ExternalFile(external) if config.full_path(&external.uses).is_yaml() => {
                vec![external.uses.to_owned()]
            }
            Self::SingleSh(_) | Self::ComplexSh { .. } | Self::ExternalFile { .. } => vec![],
        }
    }
}

#[cfg(feature = "all")]
impl<'a> EvalObject<'a> for Step {
    fn eval_object<RCtx: ReadonlyRuntimeExprContext<'a>, WCtx: WritableRuntimeExprContext>(
        &'a self,
        path: &mut Peekable<Pairs<'_, Rule>>,
        _rctx: &'a RCtx,
        _wctx: &'a WCtx,
    ) -> Result<ExprValue<'a>> {
        let Some(object) = path.next() else {
            bail!("no object path present");
        };
        let value = match self {
            Self::SingleSh(_) => {
                bail!("invalid expression for step");
            }

            Self::ExternalFile(_) => {
                // TODO: Remove once external section is removed.
                bail!("invalid expression for step");
            }

            Self::ComplexSh(command) => match object.as_span().as_str() {
                "name" => command.name.as_ref().map(|x| x.as_str()).unwrap_or(""),
                "working_dir" => command
                    .working_dir
                    .as_ref()
                    .map(|x| x.as_str())
                    .unwrap_or(""),
                "run" => &command.run,
                value => bail!("invalid steps field: {value}"),
            },
        };
        Ok(ExprValue::Text(ExprText::Ref(value)))
    }
}

#[cfg(feature = "all")]
impl<'a> Validate<'a> for Step {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        match self {
            Step::SingleSh(sh) => {
                debug!("Step is a single shell command");
                ctx.validate_symbols(sh);
            }

            Step::ComplexSh(complex) => {
                debug!("Step is a complex shell command");
                if let Some(id) = complex.id.as_ref() {
                    ctx.push_section(id);
                }

                if let Some(name) = complex.name.as_ref() {
                    ctx.push_section(name);
                }

                if let Some(wd) = complex.working_dir.as_ref() {
                    debug!("Validating step's working directory");
                    ctx.push_section("working_dir");
                    ctx.validate_symbols(wd);
                    ctx.pop_section();
                }

                debug!("Validating step's run command");
                ctx.validate_symbols(&complex.run);

                if complex.name.is_some() {
                    ctx.pop_section();
                }
            }

            Step::ExternalFile(external) => {
                debug!("Step is an external file");
                external.validate(ctx).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        action::v3::Action, expr::v3::{
            context::{CommonReadonlyRuntimeExprContext, CommonWritableRuntimeExprContext},
            exec::CommonExprExecutor,
            traits::{EvalExpr, ExprText, ExprValue},
        }, external::v3::External, pipeline::v3::Pipeline, step::v3::{ShellCommand, Step}
    };

    #[test]
    pub fn jobs_complex_step_expr_eval_success() {
        let mut wctx = CommonWritableRuntimeExprContext::default();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert(
            "main".to_string(),
            vec![
                Step::ComplexSh(Box::new(ShellCommand {
                    id: Some("second".to_string()),
                    name: Some("second_name".to_string()),
                    working_dir: Some("some_second_working_directory".to_string()),
                    run: "second_run_command".to_string(),
                })),
                Step::ComplexSh(Box::new(ShellCommand {
                    id: Some("third".to_string()),
                    name: Some("third_name".to_string()),
                    working_dir: Some("some_third_working_directory".to_string()),
                    run: "third_run_command".to_string(),
                })),
            ],
        );
        pipeline.jobs.insert(
            "backup".to_string(),
            vec![Step::ComplexSh(Box::new(ShellCommand {
                id: Some("first".to_string()),
                name: Some("first_name".to_string()),
                working_dir: Some("some_first_working_directory".to_string()),
                run: "first_run_command".to_string(),
            }))],
        );
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);

        let actual = exec.eval("${{ jobs.main.second.name }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("second_name"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ jobs.main.third.name }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("third_name"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ jobs.backup.first.name }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("first_name"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ jobs.main.second.working_dir }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref(
                "some_second_working_directory"
            ))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ jobs.main.third.working_dir }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref(
                "some_third_working_directory"
            ))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ jobs.backup.first.working_dir }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref(
                "some_first_working_directory"
            ))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ jobs.main.second.run }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("second_run_command"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ jobs.main.third.run }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("third_run_command"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ jobs.backup.first.run }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("first_run_command"))),
            Ok(ExprValue::Boolean(true))
        ));
    }

    #[test]
    pub fn steps_complex_step_expr_eval_success() {
        let mut wctx = CommonWritableRuntimeExprContext::default();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut action = Action::default();
        action.steps.push(
            Step::ComplexSh(Box::new(ShellCommand {
                id: Some("second".to_string()),
                name: Some("second_name".to_string()),
                working_dir: Some("some_second_working_directory".to_string()),
                run: "second_run_command".to_string(),
            }))
        );
        action.steps.push(
            Step::ComplexSh(Box::new(ShellCommand {
                id: Some("third".to_string()),
                name: Some("third_name".to_string()),
                working_dir: Some("some_third_working_directory".to_string()),
                run: "third_run_command".to_string(),
            }))
        );
        action.steps.push(
            Step::ComplexSh(Box::new(ShellCommand {
                id: Some("first".to_string()),
                name: Some("first_name".to_string()),
                working_dir: Some("some_first_working_directory".to_string()),
                run: "first_run_command".to_string(),
            }))
        );

        let exec = CommonExprExecutor::new(&action, &rctx, &mut wctx);

        let actual = exec.eval("${{ steps.second.name }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("second_name"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ steps.third.name }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("third_name"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ steps.first.name }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("first_name"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ steps.second.working_dir }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref(
                "some_second_working_directory"
            ))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ steps.third.working_dir }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref(
                "some_third_working_directory"
            ))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ steps.first.working_dir }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref(
                "some_first_working_directory"
            ))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ steps.second.run }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("second_run_command"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ steps.third.run }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("third_run_command"))),
            Ok(ExprValue::Boolean(true))
        ));

        let actual = exec.eval("${{ steps.first.run }}").unwrap();
        assert!(matches!(
            actual.try_eq(&ExprValue::Text(ExprText::Ref("first_run_command"))),
            Ok(ExprValue::Boolean(true))
        ));
    }

    #[test]
    pub fn jobs_simple_step_expr_eval_success() {
        let mut wctx = CommonWritableRuntimeExprContext::default();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert(
            "main".to_string(),
            vec![
                Step::SingleSh("first_run_command".to_string()),
                Step::SingleSh("second_run_command".to_string()),
            ],
        );
        pipeline.jobs.insert(
            "backup".to_string(),
            vec![Step::SingleSh("third_run_command".to_string())],
        );
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);

        let actual = exec.eval("${{ jobs.main.first }}");
        assert!(actual.is_err());

        let actual = exec.eval("${{ jobs.main.second }}");
        assert!(actual.is_err());

        let actual = exec.eval("${{ jobs.backup.third }}");
        assert!(actual.is_err());
    }

    #[test]
    pub fn steps_simple_step_expr_eval_success() {
        let mut wctx = CommonWritableRuntimeExprContext::default();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut action = Action::default();
        action.steps.push(
            Step::SingleSh("first_run_command".to_string())
        );
        action.steps.push(
            Step::SingleSh("second_run_command".to_string())
        );
        action.steps.push(
            Step::SingleSh("third_run_command".to_string())
        );
        let exec = CommonExprExecutor::new(&action, &rctx, &mut wctx);

        let actual = exec.eval("${{ steps.first }}");
        assert!(actual.is_err());

        let actual = exec.eval("${{ steps.second }}");
        assert!(actual.is_err());

        let actual = exec.eval("${{ steps.third }}");
        assert!(actual.is_err());
    }

    #[test]
    pub fn jobs_external_step_expr_eval_success() {
        let mut wctx = CommonWritableRuntimeExprContext::default();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert(
            "main".to_string(),
            vec![
                Step::ExternalFile(Box::new(External::default())),
                Step::ExternalFile(Box::new(External::default())),
            ],
        );
        pipeline.jobs.insert(
            "backup".to_string(),
            vec![Step::ExternalFile(Box::new(External::default()))],
        );
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);

        let actual = exec.eval("${{ jobs.main.first }}");
        assert!(actual.is_err());

        let actual = exec.eval("${{ jobs.main.second }}");
        assert!(actual.is_err());

        let actual = exec.eval("${{ jobs.backup.third }}");
        assert!(actual.is_err());
    }

    #[test]
    pub fn steps_external_step_expr_eval_success() {
        let mut wctx = CommonWritableRuntimeExprContext::default();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut action = Action::default();
        action.steps.push(
            Step::ExternalFile(Box::new(External::default()))
        );
        action.steps.push(
            Step::ExternalFile(Box::new(External::default()))
        );
        action.steps.push(
            Step::ExternalFile(Box::new(External::default()))
        );
        let exec = CommonExprExecutor::new(&action, &rctx, &mut wctx);

        let actual = exec.eval("${{ steps.main.first }}");
        assert!(actual.is_err());

        let actual = exec.eval("${{ steps.main.second }}");
        assert!(actual.is_err());

        let actual = exec.eval("${{ steps.backup.third }}");
        assert!(actual.is_err());
    }
}
