use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::{
    inputs::v3::Input,
    step::v3::Step,
    traits::{IntoVariables, Variables},
};

#[cfg(feature = "all")]
use std::iter::Peekable;

#[cfg(feature = "all")]
use anyhow::{Result, anyhow, bail};

#[cfg(feature = "all")]
use bld_core::fs::FileSystem;

#[cfg(feature = "all")]
use bld_config::{
    BldConfig,
    definitions::{
        KEYWORD_BLD_DIR_V3, KEYWORD_PROJECT_DIR_V3, KEYWORD_RUN_PROPS_ID_V3,
        KEYWORD_RUN_PROPS_START_TIME_V3,
    },
};

#[cfg(feature = "all")]
use crate::{
    expr::v3::{
        parser::Rule,
        traits::{
            EvalObject, ExprText, ExprValue, ReadonlyRuntimeExprContext, WritableRuntimeExprContext,
        },
    },
    traits::Dependencies,
    validator::v3::{Validate, ValidatorContext},
};

#[cfg(feature = "all")]
use tracing::debug;

#[cfg(feature = "all")]
use pest::iterators::Pairs;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Action {
    pub name: String,

    #[serde(default)]
    pub inputs: HashMap<String, Input>,

    #[serde(default)]
    pub steps: Vec<Step>,
}

impl Action {
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
            let map = self
                .inputs
                .iter()
                .filter(|(_, v)| v.is_required())
                .map(|(k, _)| k.as_str())
                .collect();
            Some(map)
        } else {
            None
        }
    }
}

impl IntoVariables for Action {
    fn into_variables(self) -> Variables {
        let mut inputs = HashMap::new();
        for (name, input) in self.inputs {
            match input {
                Input::Simple(v) => {
                    inputs.insert(name, v);
                }
                Input::Complex { default, .. } => {
                    inputs.insert(name, default.unwrap_or_default());
                }
            }
        }
        (Some(inputs), None)
    }
}

#[cfg(feature = "all")]
impl Dependencies for Action {
    async fn local_deps(&self, config: &BldConfig, fs: &FileSystem) -> Vec<String> {
        let mut dependecies = vec![];
        for step in &self.steps {
            let mut local_deps = step.local_deps(config, fs).await;
            dependecies.append(&mut local_deps);
        }
        dependecies
    }
}

#[cfg(feature = "all")]
impl<'a> EvalObject<'a> for Action {
    fn eval_object<RCtx: ReadonlyRuntimeExprContext<'a>, WCtx: WritableRuntimeExprContext>(
        &'a self,
        path: &mut Peekable<Pairs<'a, Rule>>,
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
            "name" => Ok(ExprValue::Text(ExprText::Ref(self.name.as_str()))),

            "inputs" => {
                let Some(part) = object_parts.next() else {
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

            "steps" => {
                let Some(step_id) = object_parts.next() else {
                    bail!("expected id for step in expression");
                };

                let step_id = step_id.as_span().as_str();
                let Some(step) = self.steps.iter().find(|x| x.is(step_id)) else {
                    bail!("step with id {step_id} not defined");
                };

                step.eval_object(&mut object_parts.peekable(), rctx, wctx)
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

            value => bail!("invalid expression identifier {value}"),
        }
    }
}

#[cfg(feature = "all")]
impl<'a> Validate<'a> for Action {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        debug!("Validating action: {}", self.name);

        debug!("Validating action's name value");
        ctx.push_section("name");
        ctx.validate_expressions(&self.name);
        ctx.pop_section();

        debug!("Validating action's inputs section");
        ctx.push_section("inputs");
        for (name, input) in self.inputs.iter() {
            debug!("Validating input: {}", name);
            ctx.push_section(name);
            input.validate(ctx).await;
            ctx.pop_section();
        }
        ctx.pop_section();

        debug!("Validating action's steps");
        ctx.push_section("steps");
        for (i, step) in self.steps.iter().enumerate() {
            debug!("Validating step at index {i}");
            step.validate(ctx).await;
        }
        ctx.pop_section();
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

    use super::Action;

    #[test]
    pub fn name_expr_eval_success() {
        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut action = Action::default();
        let data = vec!["test", "hello world", ""];

        for entry in data {
            action.name = entry.to_string();

            let exec = CommonExprExecutor::new(&action, &rctx, &wctx);
            let Ok(value) = exec.eval("${{ name }}") else {
                panic!("result is an error during expression evaluation");
            };

            let expected = ExprValue::Text(ExprText::Ref(entry));

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
        let mut action = Action::default();
        action
            .inputs
            .insert("name".to_string(), Input::Simple("john".to_string()));
        action
            .inputs
            .insert("surname".to_string(), Input::Simple("doe".to_string()));
        action
            .inputs
            .insert("age".to_string(), Input::Simple("30".to_string()));
        action.inputs.insert(
            "address".to_string(),
            Input::Complex {
                default: Some("highway".to_string()),
                description: None,
                required: false,
            },
        );
        action.inputs.insert(
            "taxId".to_string(),
            Input::Complex {
                default: Some("999999999".to_string()),
                description: Some("a test input".to_string()),
                required: true,
            },
        );

        let exec = CommonExprExecutor::new(&action, &rctx, &wctx);

        for (k, v) in &action.inputs {
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
        let action = Action::default();

        let exec = CommonExprExecutor::new(&action, &rctx, &wctx);

        for (name, expected) in data {
            let expr = format!("{} inputs.{name} {}", "${{", "}}");
            let actual = exec.eval(&expr).unwrap();
            assert!(matches!(
                actual.try_eq(&ExprValue::Text(ExprText::Ref(&expected))),
                Ok(ExprValue::Boolean(true))
            ));
        }
    }
}
