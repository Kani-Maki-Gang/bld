use crate::{
    inputs::v3::Input,
    job::v3::Job,
    traits::{IntoVariables, Variables},
};
#[cfg(feature = "all")]
use bld_core::fs::FileSystem;
#[cfg(feature = "all")]
use bld_pkg::PackageManager;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[cfg(feature = "all")]
use {
    crate::{
        deps::v3::{Dependencies, Dependency},
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
    bld_config::definitions::{
        KEYWORD_BLD_DIR_V3, KEYWORD_PROJECT_DIR_V3, KEYWORD_RUN_PROPS_ID_V3,
        KEYWORD_RUN_PROPS_START_TIME_V3,
    },
    cron::Schedule,
    pest::iterators::Pairs,
    std::{iter::Peekable, str::FromStr},
    tracing::debug,
};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub name: Option<String>,

    pub cron: Option<String>,

    #[serde(default)]
    pub env: HashMap<String, String>,

    #[serde(default)]
    pub inputs: HashMap<String, Input>,

    #[serde(default)]
    pub jobs: HashMap<String, Job>,
}

impl Pipeline {
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
        ctx.push_section("cron");
        if ctx.contains_expressions(cron) {
            ctx.validate_expressions(cron);
        } else if let Err(e) = Schedule::from_str(cron) {
            let error = format!("{cron} {e}");
            ctx.append_error(&error);
        }
        ctx.pop_section();
    }

    #[cfg(feature = "all")]
    async fn validate_jobs<'a, C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        ctx.push_section("jobs");

        if self.jobs.is_empty() {
            ctx.append_error("Pipeline must have at least one job defined");
        }

        let mut job_ids = HashSet::new();
        for (name, job) in &self.jobs {
            ctx.push_job_section(name);
            debug!("Validating {name} job's steps");
            let job_id = &job.id;
            if !job_ids.insert(job_id) {
                ctx.push_section(job_id);
                ctx.append_error(&format!("Duplicate job id '{job_id}' found"));
                ctx.pop_section();
            }
            job.validate(ctx).await;
            ctx.pop_section();
        }
        ctx.pop_section();
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
impl<'a> Dependencies<'a> for Pipeline {
    async fn local_deps(
        &'a self,
        manager: &PackageManager,
        fs: &FileSystem,
    ) -> Vec<Dependency<'a>> {
        let mut dependecies = vec![];
        for job in self.jobs.values() {
            dependecies.append(&mut job.local_deps(manager, fs).await);
        }
        dependecies
    }

    async fn remote_deps(&'a self) -> Vec<Dependency<'a>> {
        let mut dependecies = vec![];
        for job in self.jobs.values() {
            dependecies.append(&mut job.remote_deps().await);
        }
        dependecies
    }

    async fn jobs(&'a self) -> Vec<Dependency<'a>> {
        let mut set = HashSet::new();
        for name in self.jobs.keys() {
            set.insert(name.as_str());
        }
        set.into_iter().map(Dependency::Job).collect()
    }

    async fn all(&'a self, manager: &PackageManager, fs: &FileSystem) -> Vec<Dependency<'a>> {
        let mut deps = self.local_deps(manager, fs).await;
        deps.append(&mut self.remote_deps().await);
        deps
    }
}

#[cfg(feature = "all")]
impl<'a> EvalObject<'a> for Pipeline {
    fn eval_object<RCtx: ReadonlyRuntimeExprContext<'a>, WCtx: WritableRuntimeExprContext>(
        &'a self,
        path: &mut Peekable<Pairs<'a, Rule>>,
        rctx: &'a RCtx,
        wctx: &'a WCtx,
    ) -> Result<ExprValue<'a>> {
        let Some(object) = path.next() else {
            bail!("no object path present");
        };

        let mut object_parts = object.into_inner().peekable();
        let Some(part) = object_parts.peek() else {
            bail!("expected at least one part in the object path");
        };

        match part.as_span().as_str() {
            "name" => {
                let name = self.name.as_ref().map_or("", |x| x.as_str());
                Ok(ExprValue::Text(ExprText::Ref(name)))
            }

            "cron" => {
                let cron = self.cron.as_ref().map_or("", |x| x.as_str());
                Ok(ExprValue::Text(ExprText::Ref(cron)))
            }

            "inputs" => {
                let Some(part) = object_parts.nth(1) else {
                    bail!("expected name of input in object path");
                };
                let name = part.as_span().as_str();

                let input = rctx.get_input(name).or_else(|_| {
                    self.inputs
                        .get(name)
                        .ok_or_else(|| anyhow!("input '{name}' not found"))
                        .and_then(|x| x.try_into())
                });

                input.map(|x| ExprValue::Text(ExprText::Ref(x)))
            }

            "env" => {
                let Some(part) = object_parts.nth(1) else {
                    bail!("expected name of env variable in object path");
                };
                let name = part.as_span().as_str();
                rctx.get_env(name)
                    .or_else(|_| {
                        self.env
                            .get(name)
                            .map(|x| x.as_str())
                            .ok_or_else(|| anyhow!("env variable '{name}' not found"))
                    })
                    .map(|x| ExprValue::Text(ExprText::Ref(x)))
            }

            // Keywords section
            value if value == KEYWORD_BLD_DIR_V3 => {
                Ok(ExprValue::Text(ExprText::Ref(rctx.get_root_dir())))
            }

            value if value == KEYWORD_PROJECT_DIR_V3 => {
                Ok(ExprValue::Text(ExprText::Ref(rctx.get_project_dir())))
            }

            value if value == KEYWORD_RUN_PROPS_ID_V3 => {
                Ok(ExprValue::Text(ExprText::Ref(rctx.get_run_id())))
            }

            value if value == KEYWORD_RUN_PROPS_START_TIME_V3 => {
                Ok(ExprValue::Text(ExprText::Ref(rctx.get_run_start_time())))
            }

            // Move evaluation to the job level
            _ => {
                let Some(job) = wctx.get_exec_id().and_then(|id| self.jobs.get(id)) else {
                    bail!("unable to find executing job id");
                };
                job.eval_object(&mut object_parts, rctx, wctx)
            },
        }
    }
}

#[cfg(feature = "all")]
impl<'a> Validate<'a> for Pipeline {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        debug!("Validating pipeline");

        if let Some(name) = self.name.as_ref() {
            debug!("Validating pipeline's name value");
            ctx.push_section("name");
            ctx.validate_expressions(name);
            ctx.pop_section();
        }

        debug!("Validating pipeline's cron value");
        self.validate_cron(ctx);

        debug!("Validating pipeline's inputs section");
        ctx.push_section("inputs");
        for (name, input) in self.inputs.iter() {
            debug!("Validating input: {}", name);
            ctx.push_section(name);
            input.validate(ctx).await;
            ctx.pop_section();
        }
        ctx.pop_section();

        debug!("Validating pipeline's env section");
        ctx.push_section("env");
        ctx.validate_env(&self.env);
        ctx.pop_section();

        debug!("Validating pipeline's jobs section");
        self.validate_jobs(ctx).await;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use bld_utils::sync::IntoArc;

    use crate::{
        expr::v3::{
            context::CommonReadonlyRuntimeExprContext,
            exec::CommonExprExecutor,
            traits::{EvalExpr, ExprText, ExprValue, MockWritableRuntimeExprContext},
        },
        inputs::v3::Input,
    };

    use super::Pipeline;

    #[test]
    pub fn name_expr_eval_success() {
        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        let data = vec![Some("test"), Some("hello world"), Some(""), None];

        for entry in data {
            pipeline.name = entry.map(|x| x.to_string());

            let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);
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
        let wctx = MockWritableRuntimeExprContext::new();
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

            let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);
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
    pub fn env_expr_eval_success() {
        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        pipeline.env.insert("NODE".to_string(), "22.10".to_string());
        pipeline.env.insert("PATH".to_string(), "value".to_string());
        pipeline
            .env
            .insert("HOME".to_string(), "/home/user".to_string());

        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        for (k, v) in &pipeline.env {
            let expr = format!("{} env.{k} {}", "${{", "}}");
            let Ok(value) = exec.eval(&expr) else {
                panic!("result is an error during expression evaluation");
            };
            let expected = ExprValue::Text(ExprText::Ref(v));
            assert!(matches!(
                value.try_eq(&expected),
                Ok(ExprValue::Boolean(true))
            ));
        }
    }

    #[test]
    pub fn inputs_expr_eval_success() {
        let wctx = MockWritableRuntimeExprContext::new();
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

        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

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

    #[test]
    pub fn inputs_use_rctx_expr_eval_success() {
        // Arrange
        let data: HashMap<String, String> = vec![
            ("name", "john"),
            ("surname", "doe"),
            ("age", "30"),
            ("address", "some address"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext {
            inputs: data.clone().into_arc(),
            ..Default::default()
        };
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        for (name, expected) in data {
            let expr = format!("{} inputs.{name} {}", "${{", "}}");
            // Act
            let actual = exec.eval(&expr).unwrap();

            // Assert
            assert!(matches!(
                actual.try_eq(&ExprValue::Text(ExprText::Ref(&expected))),
                Ok(ExprValue::Boolean(true))
            ));
        }
    }

    #[test]
    pub fn env_use_rctx_expr_eval_success() {
        // Arrange
        let data: HashMap<String, String> = vec![
            ("name", "john"),
            ("surname", "doe"),
            ("age", "30"),
            ("address", "some address"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext {
            env: data.clone().into_arc(),
            ..Default::default()
        };
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        for (name, expected) in data {
            let expr = format!("{} env.{name} {}", "${{", "}}");
            // Act
            let actual = exec.eval(&expr).unwrap();

            // Assert
            assert!(matches!(
                actual.try_eq(&ExprValue::Text(ExprText::Ref(&expected))),
                Ok(ExprValue::Boolean(true))
            ));
        }
    }
}
