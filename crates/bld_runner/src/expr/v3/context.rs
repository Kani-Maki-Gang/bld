use crate::expr::v3::exec::{CommonExprExecutor, eval_all_expressions};
use crate::expr::v3::parser;
use crate::expr::v3::traits::EvalObject;
use crate::inputs::v3::Input;

use super::traits::{
    ExprValue, OutputScope, ReadonlyRuntimeExprContext, WritableRuntimeExprContext,
};
use anyhow::{Context, Error, Result, anyhow};
use bld_config::BldConfig;
use bld_config::definitions::{
    KEYWORD_BLD_DIR_V3, KEYWORD_PROJECT_DIR_V3, KEYWORD_RUN_PROPS_ID_V3,
    KEYWORD_RUN_PROPS_START_TIME_V3,
};
use bld_utils::sync::IntoArc;
use std::{collections::HashMap, sync::Arc};

pub struct StartOfRunWritableExprContext;

pub static START_OF_RUN_WCTX: StartOfRunWritableExprContext = StartOfRunWritableExprContext;

pub fn out_of_scope(symbol: &str) -> Error {
    anyhow!(
        "'{symbol}' is not available at the start of a run, only inputs, env, {KEYWORD_BLD_DIR_V3}, {KEYWORD_PROJECT_DIR_V3}, {KEYWORD_RUN_PROPS_ID_V3} and {KEYWORD_RUN_PROPS_START_TIME_V3} can be used here"
    )
}

impl WritableRuntimeExprContext for StartOfRunWritableExprContext {
    fn get_exec_id(&self) -> Option<&str> {
        None
    }

    fn get_output<'a>(&'a self, scope: OutputScope, id: &str, name: &str) -> Result<ExprValue<'a>> {
        match scope {
            OutputScope::Step => Err(out_of_scope(&format!("steps.{id}.outputs.{name}"))),
            OutputScope::Job => Err(out_of_scope(&format!("jobs.{id}.outputs.{name}"))),
        }
    }

    fn set_output(&mut self, _id: &str, name: String, _value: String) -> Result<()> {
        Err(out_of_scope(&format!("outputs.{name}")))
    }

    fn set_outputs(&mut self, _id: &str, _outputs: HashMap<String, String>) -> Result<()> {
        Err(out_of_scope("outputs"))
    }

    fn get_matrix_value<'a>(&'a self, name: &str) -> Result<&'a str> {
        Err(out_of_scope(&format!("matrix.{name}")))
    }
}

#[derive(Clone, Debug, Default)]
pub struct CommonReadonlyRuntimeExprContext {
    pub config: Arc<BldConfig>,
    pub inputs: Arc<HashMap<String, String>>,
    pub env: Arc<HashMap<String, String>>,
    pub run_id: String,
    pub run_start_time: String,
}

impl CommonReadonlyRuntimeExprContext {
    pub fn new(
        config: Arc<BldConfig>,
        inputs: Arc<HashMap<String, String>>,
        env: Arc<HashMap<String, String>>,
        run_id: String,
        run_start_time: String,
    ) -> Self {
        Self {
            config,
            inputs,
            env,
            run_id,
            run_start_time,
        }
    }

    pub fn clone_with(&self, closure: impl FnOnce(&mut Self) -> ()) -> Self {
        let mut clone = self.clone();
        closure(&mut clone);
        clone
    }
}

impl<'a> ReadonlyRuntimeExprContext<'a> for CommonReadonlyRuntimeExprContext {
    fn get_root_dir(&'a self) -> &'a str {
        &self.config.root_dir
    }

    fn get_project_dir(&'a self) -> &'a str {
        &self.config.project_dir
    }

    fn get_input(&'a self, name: &'a str) -> Result<&'a str> {
        self.inputs
            .get(name)
            .map(|x| x.as_str())
            .ok_or_else(|| anyhow!("input '{name}' not found"))
    }

    fn get_env(&'a self, name: &'a str) -> Result<&'a str> {
        self.env
            .get(name)
            .map(|x| x.as_str())
            .ok_or_else(|| anyhow!("env variable '{name}' not found"))
    }

    fn get_run_id(&'a self) -> &'a str {
        &self.run_id
    }

    fn get_run_start_time(&'a self) -> &'a str {
        &self.run_start_time
    }
}

pub struct CommonReadonlyRuntimeExprContextOptions<'a, T: for<'x> EvalObject<'x>> {
    pub obj: &'a T,
    pub config: Arc<BldConfig>,
    pub inputs: Arc<HashMap<String, String>>,
    pub declared_inputs: &'a HashMap<String, Input>,
    pub env: Arc<HashMap<String, String>>,
    pub declared_env: &'a HashMap<String, String>,
    pub run_id: String,
    pub run_start_time: String,
}

pub fn expr_rctx<'a, T: for<'x> EvalObject<'x>>(
    options: CommonReadonlyRuntimeExprContextOptions<'a, T>,
) -> Result<CommonReadonlyRuntimeExprContext> {
    let regex = parser::new_regex()?;
    let mut inputs = (*options.inputs).clone();
    let mut env = (*options.env).clone();

    // Supplied values always take precedence over the ones declared in the file.
    let rctx = CommonReadonlyRuntimeExprContext::new(
        options.config.clone(),
        options.inputs.clone(),
        options.env.clone(),
        options.run_id.clone(),
        options.run_start_time.clone(),
    );
    let exec = CommonExprExecutor::new(options.obj, &rctx, &START_OF_RUN_WCTX);
    for (name, input) in options.declared_inputs {
        let Some(raw) = input.default_value().filter(|_| !inputs.contains_key(name)) else {
            continue;
        };
        let value = eval_all_expressions(&exec, &regex, raw)
            .with_context(|| format!("unable to resolve inputs.{name} at the start of the run"))?;
        inputs.insert(name.to_string(), value);
    }

    let rctx = CommonReadonlyRuntimeExprContext::new(
        options.config.clone(),
        inputs.clone().into_arc(),
        env.clone().into_arc(),
        options.run_id.clone(),
        options.run_start_time.clone(),
    );
    let exec = CommonExprExecutor::new(options.obj, &rctx, &START_OF_RUN_WCTX);
    for (name, raw) in options.declared_env {
        if env.contains_key(name) {
            continue;
        }
        let value = eval_all_expressions(&exec, &regex, raw)
            .with_context(|| format!("unable to resolve env.{name} at the start of the run"))?;
        env.insert(name.to_string(), value);
    }

    Ok(CommonReadonlyRuntimeExprContext::new(
        options.config,
        inputs.into_arc(),
        env.into_arc(),
        options.run_id,
        options.run_start_time,
    ))
}

#[cfg(test)]
mod tests {
    use bld_config::BldConfig;

    use crate::{expr::v3::traits::ReadonlyRuntimeExprContext, pipeline::v3::Pipeline};

    use super::*;

    fn complex_input(default: &str) -> Input {
        Input::Complex {
            description: None,
            default: Some(default.to_string()),
            required: false,
        }
    }

    fn resolve(
        pipeline: &Pipeline,
        supplied_inputs: Vec<(&str, &str)>,
    ) -> Result<CommonReadonlyRuntimeExprContext> {
        let config = BldConfig {
            project_dir: "/home/user/project".to_string(),
            ..Default::default()
        };
        let supplied_inputs: HashMap<String, String> = supplied_inputs
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        expr_rctx(CommonReadonlyRuntimeExprContextOptions {
            obj: pipeline,
            config: config.into_arc(),
            inputs: supplied_inputs.into_arc(),
            declared_inputs: &pipeline.inputs,
            env: HashMap::new().into_arc(),
            declared_env: &pipeline.env,
            run_id: "run-id".to_string(),
            run_start_time: "start-time".to_string(),
        })
    }

    #[test]
    pub fn input_default_with_keyword_resolve_success() {
        let mut pipeline = Pipeline::default();
        pipeline.inputs.insert(
            "worktree_root".to_string(),
            complex_input("${{ bld_project_dir }}/../worktrees/bld"),
        );

        let rctx = resolve(&pipeline, vec![]).unwrap();

        assert_eq!(
            rctx.get_input("worktree_root").unwrap(),
            "/home/user/project/../worktrees/bld"
        );
    }

    #[test]
    pub fn env_referencing_input_resolve_success() {
        let mut pipeline = Pipeline::default();
        pipeline.inputs.insert(
            "repo_dir".to_string(),
            complex_input("${{ bld_project_dir }}/repo"),
        );
        pipeline.env.insert(
            "LOGS".to_string(),
            "${{ inputs.repo_dir }}/logs".to_string(),
        );

        let rctx = resolve(&pipeline, vec![]).unwrap();

        assert_eq!(
            rctx.get_input("repo_dir").unwrap(),
            "/home/user/project/repo"
        );
        assert_eq!(
            rctx.get_env("LOGS").unwrap(),
            "/home/user/project/repo/logs"
        );
    }

    #[test]
    pub fn input_default_referencing_another_default_resolve_failure() {
        let mut pipeline = Pipeline::default();
        pipeline
            .inputs
            .insert("first".to_string(), complex_input("/root"));
        pipeline.inputs.insert(
            "second".to_string(),
            complex_input("${{ inputs.first }}/sub"),
        );

        let error = format!("{:#}", resolve(&pipeline, vec![]).unwrap_err());

        assert!(error.contains("unable to resolve inputs.second"), "{error}");
    }

    #[test]
    pub fn supplied_input_overrides_default_resolve_success() {
        let mut pipeline = Pipeline::default();
        // An unresolvable default is fine as long as the input is supplied.
        pipeline.inputs.insert(
            "image".to_string(),
            complex_input("${{ steps.build.outputs.image }}"),
        );

        let rctx = resolve(&pipeline, vec![("image", "ubuntu:22.04")]).unwrap();

        assert_eq!(rctx.get_input("image").unwrap(), "ubuntu:22.04");
    }

    #[test]
    pub fn input_without_default_is_left_unresolved() {
        let mut pipeline = Pipeline::default();
        pipeline.inputs.insert(
            "image".to_string(),
            Input::Complex {
                description: None,
                default: None,
                required: true,
            },
        );

        let rctx = resolve(&pipeline, vec![]).unwrap();

        assert!(rctx.get_input("image").is_err());
    }

    #[test]
    pub fn runtime_expr_in_start_of_run_values_resolve_failure() {
        let data = [
            "${{ steps.build.outputs.image }}",
            "${{ matrix.os }}",
            "${{ runs_on }}",
        ];

        for value in data {
            let mut pipeline = Pipeline::default();
            pipeline
                .inputs
                .insert("image".to_string(), complex_input(value));

            let error = format!("{:#}", resolve(&pipeline, vec![]).unwrap_err());

            assert!(
                error.contains("is not available at the start of a run"),
                "{error}"
            );
        }
    }
}
