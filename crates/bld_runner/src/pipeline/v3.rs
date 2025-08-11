use crate::{
    external::v3::External,
    inputs::v3::Input,
    runs_on::v3::RunsOn,
    step::v3::Step,
    traits::{IntoVariables, Variables},
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[cfg(feature = "all")]
use std::iter::Peekable;

#[cfg(feature = "all")]
use {
    crate::{
        expr::v3::{
            parser::Rule,
            traits::{
                EvalObject, ExprText, ExprValue, ReadonlyRuntimeExprContext,
                WritableRuntimeExprContext,
            },
        },
        validator::v3::{Validate, ValidatorContext},
    },
    anyhow::{Result, anyhow, bail},
    cron::Schedule,
    pest::iterators::Pairs,
    std::str::FromStr,
    tracing::debug,
};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub name: Option<String>,
    pub runs_on: RunsOn,
    pub cron: Option<String>,

    #[serde(default = "Pipeline::default_dispose")]
    pub dispose: bool,

    #[serde(default)]
    pub env: HashMap<String, String>,

    #[serde(default)]
    pub inputs: HashMap<String, Input>,

    #[serde(default)]
    pub external: Vec<External>,

    #[serde(default)]
    pub jobs: HashMap<String, Vec<Step>>,
}

impl Pipeline {
    fn default_dispose() -> bool {
        true
    }

    pub fn inputs_map(&self) -> HashMap<String, String> {
        let mut inputs = HashMap::new();
        for (name, input) in &self.inputs {
            match input {
                Input::Simple(v) => {
                    inputs.insert(name.to_owned(), v.to_owned());
                }
                Input::Complex { default, .. } => {
                    inputs.insert(name.to_owned(), default.to_owned().unwrap_or_default());
                }
            }
        }
        inputs
    }

    pub fn required_inputs(&self) -> Option<HashSet<&str>> {
        if !self.inputs.is_empty() {
            let inputs = self
                .inputs
                .iter()
                .filter(|(_, v)| v.is_required())
                .map(|(k, _)| k.as_str())
                .collect();
            Some(inputs)
        } else {
            None
        }
    }

    #[cfg(feature = "all")]
    fn validate_cron<'a, C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        let Some(cron) = self.cron.as_ref() else {
            return;
        };
        if let Err(e) = Schedule::from_str(cron) {
            let error = format!("{cron} {e}");
            ctx.push_section("cron");
            ctx.append_error(&error);
            ctx.pop_section();
        }
    }
}

impl IntoVariables for Pipeline {
    fn into_variables(self) -> Variables {
        let mut inputs: Option<HashMap<String, String>> = None;

        if !self.inputs.is_empty() {
            let map = self
                .inputs
                .into_iter()
                .map(|(name, input)| match input {
                    Input::Simple(v) => (name, v),
                    Input::Complex { default, .. } => (name, default.unwrap_or_default()),
                })
                .collect();
            inputs = Some(map);
        }

        (inputs, Some(self.env))
    }
}

#[cfg(feature = "all")]
impl<'a> EvalObject<'a> for Pipeline {
    fn eval_object<RCtx: ReadonlyRuntimeExprContext<'a>, WCtx: WritableRuntimeExprContext>(
        &'a self,
        path: &mut Peekable<Pairs<'_, Rule>>,
        rctx: &'a RCtx,
        wctx: &'a WCtx,
    ) -> Result<ExprValue<'a>> {
        let Some(object) = path.next() else {
            bail!("no object path present");
        };

        let mut object_parts = object.into_inner();
        let Some(part) = object_parts.next() else {
            bail!("expected at least one part in the object path");
        };

        match part.as_span().as_str() {
            "name" => {
                let name = self.name.as_ref().map_or("", |x| x.as_str());
                Ok(ExprValue::Text(ExprText::Ref(name)))
            }

            "runs_on" => self
                .runs_on
                .eval_object(&mut object_parts.peekable(), rctx, wctx),

            "dispose" => Ok(ExprValue::Boolean(self.dispose)),

            "cron" => {
                let cron = self.cron.as_ref().map_or("", |x| x.as_str());
                Ok(ExprValue::Text(ExprText::Ref(cron)))
            }

            "inputs" => {
                let Some(part) = object_parts.next() else {
                    bail!("expected name of input in object path");
                };
                let name = part.as_span().as_str();
                let input = self
                    .inputs
                    .get(name)
                    .ok_or_else(|| anyhow!("input '{name}' not found"))?;
                input.try_into().map(|x| ExprValue::Text(ExprText::Ref(x)))
            }

            "env" => {
                let Some(part) = object_parts.next() else {
                    bail!("expected name of env variable in object path");
                };
                let name = part.as_span().as_str();
                self.env
                    .get(name)
                    .map(|x| ExprValue::Text(ExprText::Ref(x)))
                    .ok_or_else(|| anyhow!("env variable '{name}' not found"))
            }

            "jobs" => {
                let Some(job_name) = object_parts.next() else {
                    bail!("expected name for job in expression");
                };

                let Some(job) = self.jobs.get(job_name.as_span().as_str()) else {
                    bail!("job with name {job_name} not defined");
                };

                let Some(step_id) = object_parts.next() else {
                    bail!("expected id for step in expression");
                };

                let step_id = step_id.as_span().as_str();
                let Some(step) = job.iter().find(|x| x.is(step_id)) else {
                    bail!("step with id {step_id} not defined");
                };

                step.eval_object(&mut object_parts.peekable(), rctx, wctx)
            }

            value => bail!("invalid expression identifier {value}"),
        }
    }
}

#[cfg(feature = "all")]
impl<'a> Validate<'a> for Pipeline {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        debug!("Validating pipeline");

        debug!("Validating pipeline's runs_on section");
        ctx.push_section("runs_on");
        self.runs_on.validate(ctx).await;
        ctx.pop_section();

        debug!("Validating pipeline's cron value");
        self.validate_cron(ctx);

        debug!("Validating pipeline's inputs section");
        ctx.push_section("inputs");
        for (name, input) in self.inputs.iter() {
            debug!("Validating input: {}", name);
            ctx.push_section(name);
            ctx.validate_keywords(name);
            input.validate(ctx).await;
            ctx.pop_section();
        }
        ctx.pop_section();

        debug!("Validating pipeline's env section");
        ctx.push_section("env");
        ctx.validate_env(&self.env);
        ctx.pop_section();

        debug!("Validating pipeline external section");
        ctx.push_section("external");
        for external in &self.external {
            external.validate(ctx).await;
        }
        ctx.pop_section();

        debug!("Validating pipeline's jobs section");
        ctx.push_section("jobs");
        for (job, steps) in &self.jobs {
            ctx.push_section(job);
            debug!("Validating {job} job's steps");
            for step in steps {
                step.validate(ctx).await;
            }
            ctx.pop_section();
        }
        ctx.pop_section();
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        expr::v3::{
            context::{CommonReadonlyRuntimeExprContext, CommonWritableRuntimeExprContext},
            exec::CommonExprExecutor,
            traits::{EvalExpr, ExprText, ExprValue},
        },
        inputs::v3::Input,
    };

    use super::Pipeline;

    #[test]
    pub fn name_expr_eval_success() {
        let mut wctx = CommonWritableRuntimeExprContext::default();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        let data = vec![Some("test"), Some("hello world"), Some(""), None];

        for entry in data {
            pipeline.name = entry.map(|x| x.to_string());

            let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);
            let Ok(value) = exec.eval("${{ name }}") else {
                panic!("result is an error during expression evaluation");
            };

            let expected = entry
                .map(|x| ExprValue::Text(ExprText::Ref(x)))
                .unwrap_or_else(|| ExprValue::Text(ExprText::Ref("")));

            assert!(matches!(
                value.try_eq(&expected),
                Ok(ExprValue::Boolean(true))
            ));
        }
    }

    #[test]
    pub fn cron_expr_eval_success() {
        let mut wctx = CommonWritableRuntimeExprContext::default();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        let data = vec![
            Some("30 * * * * 1"),
            Some("H 5 * * 1"),
            Some("1 M * * * 2"),
            None,
        ];

        for entry in data {
            pipeline.cron = entry.map(|x| x.to_string());

            let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);
            let Ok(value) = exec.eval("${{ cron }}") else {
                panic!("result is an error during expression evaluation");
            };

            let expected = entry
                .map(|x| ExprValue::Text(ExprText::Ref(x)))
                .unwrap_or_else(|| ExprValue::Text(ExprText::Ref("")));

            assert!(matches!(
                value.try_eq(&expected),
                Ok(ExprValue::Boolean(true))
            ));
        }
    }

    #[test]
    pub fn dispose_expr_eval_success() {
        let mut wctx = CommonWritableRuntimeExprContext::default();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        let data = vec![true, false];

        for entry in data {
            pipeline.dispose = entry;

            let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);
            let Ok(value) = exec.eval("${{ dispose }}") else {
                panic!("result is an error during expression evaluation");
            };
            let expected = ExprValue::Boolean(entry);
            assert!(matches!(
                value.try_eq(&expected),
                Ok(ExprValue::Boolean(true))
            ));
        }
    }

    #[test]
    pub fn env_expr_eval_success() {
        let mut wctx = CommonWritableRuntimeExprContext::default();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        pipeline.env.insert("NODE".to_string(), "22.10".to_string());
        pipeline.env.insert("PATH".to_string(), "value".to_string());
        pipeline
            .env
            .insert("HOME".to_string(), "/home/user".to_string());

        let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);

        for (k, v) in &pipeline.env {
            let expr = format!("{} env.{k} {}", "${{", "}}");
            let Ok(value) = exec.eval(&expr) else {
                panic!("result is an error during expression evaluation");
            };
            let expected = ExprValue::Text(ExprText::Ref(&v));
            assert!(matches!(
                value.try_eq(&expected),
                Ok(ExprValue::Boolean(true))
            ));
        }
    }

    #[test]
    pub fn inputs_expr_eval_success() {
        let mut wctx = CommonWritableRuntimeExprContext::default();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        pipeline
            .inputs
            .insert("name".to_string(), Input::Simple("john".to_string()));
        pipeline
            .inputs
            .insert("surname".to_string(), Input::Simple("doe".to_string()));
        pipeline
            .inputs
            .insert("age".to_string(), Input::Simple("30".to_string()));
        pipeline.inputs.insert(
            "address".to_string(),
            Input::Complex {
                default: Some("highway".to_string()),
                description: None,
                required: false,
            },
        );
        pipeline.inputs.insert(
            "taxId".to_string(),
            Input::Complex {
                default: Some("999999999".to_string()),
                description: Some("a test input".to_string()),
                required: true,
            },
        );

        let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);

        for (k, v) in &pipeline.inputs {
            let expr = format!("{} inputs.{k} {}", "${{", "}}");
            let Ok(value) = exec.eval(&expr) else {
                panic!("result is an error during expression evaluation");
            };
            let expected = match v {
                Input::Simple(value) => ExprValue::Text(ExprText::Ref(value)),
                Input::Complex {
                    default: Some(default),
                    ..
                } => ExprValue::Text(ExprText::Ref(default)),
                _ => panic!("no value defined"),
            };
            assert!(matches!(
                value.try_eq(&expected),
                Ok(ExprValue::Boolean(true))
            ));
        }
    }
}
