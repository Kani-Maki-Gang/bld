use std::collections::HashMap;

use anyhow::{Context, Result};
use bld_utils::sync::IntoArc;

use crate::{expr::v3::parser, inputs::v3::Input};

use super::{
    context::{CommonReadonlyRuntimeExprContext, START_OF_RUN_WCTX},
    exec::{CommonExprExecutor, eval_all_expressions},
    traits::EvalObject,
};

/// Resolves the values that must be known before a run starts, meaning the defaults of any
/// input that wasn't supplied and the file's env values, and returns `supplied` extended
/// with them.
///
/// Expressions in those values can only use symbols backed by the readonly context. Inputs
/// are resolved first, against the supplied values alone, so an env value can refer to an
/// input but a default cannot refer to another default.
pub fn resolve_start_of_run_context<T>(
    obj: &T,
    declared_inputs: &HashMap<String, Input>,
    declared_env: &HashMap<String, String>,
    supplied: CommonReadonlyRuntimeExprContext,
) -> Result<CommonReadonlyRuntimeExprContext>
where
    T: for<'x> EvalObject<'x>,
{
    let regex = parser::new_regex()?;
    let config = supplied.config;
    let run_id = supplied.run_id;
    let run_start_time = supplied.run_start_time;
    let mut inputs = (*supplied.inputs).clone();
    let mut env = (*supplied.env).clone();

    let resolve = |rctx: &CommonReadonlyRuntimeExprContext, symbol: String, raw: &str| {
        let exec = CommonExprExecutor::new(obj, rctx, &START_OF_RUN_WCTX);
        eval_all_expressions(&exec, &regex, raw)
            .with_context(|| format!("unable to resolve {symbol} at the start of the run"))
    };

    // Supplied values always take precedence over the ones declared in the file.
    let rctx = CommonReadonlyRuntimeExprContext::new(
        config.clone(),
        inputs.clone().into_arc(),
        env.clone().into_arc(),
        run_id.clone(),
        run_start_time.clone(),
    );
    for (name, input) in declared_inputs {
        let Some(raw) = input.default_value().filter(|_| !inputs.contains_key(name)) else {
            continue;
        };
        let value = resolve(&rctx, format!("inputs.{name}"), raw)?;
        inputs.insert(name.to_string(), value);
    }

    let rctx = CommonReadonlyRuntimeExprContext::new(
        config.clone(),
        inputs.clone().into_arc(),
        env.clone().into_arc(),
        run_id.clone(),
        run_start_time.clone(),
    );
    for (name, raw) in declared_env {
        if env.contains_key(name) {
            continue;
        }
        let value = resolve(&rctx, format!("env.{name}"), raw)?;
        env.insert(name.to_string(), value);
    }

    Ok(CommonReadonlyRuntimeExprContext::new(
        config,
        inputs.into_arc(),
        env.into_arc(),
        run_id,
        run_start_time,
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

        resolve_start_of_run_context(
            pipeline,
            &pipeline.inputs,
            &pipeline.env,
            CommonReadonlyRuntimeExprContext::new(
                config.into_arc(),
                supplied_inputs.into_arc(),
                HashMap::new().into_arc(),
                "run-id".to_string(),
                "start-time".to_string(),
            ),
        )
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
