use std::collections::HashMap;

use anyhow::{Error, Result, bail};
use bld_utils::sync::IntoArc;
use regex::Regex;

use crate::inputs::v3::Input;

use super::{
    context::{CommonReadonlyRuntimeExprContext, START_OF_RUN_WCTX},
    exec::{CommonExprExecutor, eval_all_expressions},
    parser::EXPR_REGEX,
    traits::EvalObject,
};

/// A declared value that is resolved before the run starts.
struct Pending<'a> {
    is_input: bool,
    name: &'a str,
    raw: &'a str,
}

impl Pending<'_> {
    fn symbol(&self) -> String {
        if self.is_input {
            format!("inputs.{}", self.name)
        } else {
            format!("env.{}", self.name)
        }
    }
}

/// Resolves the values that must be known before a run starts, meaning the defaults of any
/// input that wasn't supplied and the file's env values, and returns `supplied` extended
/// with them.
///
/// Expressions in those values can only use symbols backed by the readonly context, which
/// includes other inputs and env values. Since these can be defined in any order, resolution
/// is done in passes until no more values can be resolved. Values left unresolved after a
/// pass that made no progress are either cyclic or use symbols that only exist once the run
/// is under way.
pub fn resolve_start_of_run_context<T>(
    obj: &T,
    declared_inputs: &HashMap<String, Input>,
    declared_env: &HashMap<String, String>,
    supplied: CommonReadonlyRuntimeExprContext,
) -> Result<CommonReadonlyRuntimeExprContext>
where
    T: for<'x> EvalObject<'x>,
{
    let regex = Regex::new(EXPR_REGEX)?;
    let config = supplied.config;
    let run_id = supplied.run_id;
    let run_start_time = supplied.run_start_time;
    let mut inputs = (*supplied.inputs).clone();
    let mut env = (*supplied.env).clone();

    // Supplied values always take precedence over the ones declared in the file.
    let mut pending: Vec<Pending> = declared_inputs
        .iter()
        .filter(|(name, _)| !inputs.contains_key(*name))
        .filter_map(|(name, input)| {
            input.default_value().map(|raw| Pending {
                is_input: true,
                name,
                raw,
            })
        })
        .chain(
            declared_env
                .iter()
                .filter(|(name, _)| !env.contains_key(*name))
                .map(|(name, raw)| Pending {
                    is_input: false,
                    name,
                    raw,
                }),
        )
        .collect();

    let mut last_error: Option<Error> = None;

    while !pending.is_empty() {
        let rctx = CommonReadonlyRuntimeExprContext::new(
            config.clone(),
            inputs.clone().into_arc(),
            env.clone().into_arc(),
            run_id.clone(),
            run_start_time.clone(),
        );
        let exec = CommonExprExecutor::new(obj, &rctx, &START_OF_RUN_WCTX);

        let mut resolved = Vec::new();
        let mut remaining = Vec::new();

        for entry in pending {
            match eval_all_expressions(&exec, &regex, entry.raw) {
                Ok(value) => resolved.push((entry, value)),
                Err(e) => {
                    last_error = Some(e);
                    remaining.push(entry);
                }
            }
        }

        if resolved.is_empty() {
            let mut symbols: Vec<String> = remaining.iter().map(|x| x.symbol()).collect();
            symbols.sort();
            let error = last_error.map(|e| format!(": {e}")).unwrap_or_default();
            bail!(
                "unable to resolve {} at the start of the run, check for cyclic references between them{error}",
                symbols.join(", ")
            );
        }

        for (entry, value) in resolved {
            if entry.is_input {
                inputs.insert(entry.name.to_string(), value);
            } else {
                env.insert(entry.name.to_string(), value);
            }
        }

        pending = remaining;
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
    pub fn input_default_referencing_other_values_resolve_success() {
        let mut pipeline = Pipeline::default();
        pipeline.inputs.insert(
            "repo_dir".to_string(),
            complex_input("${{ env.ROOT }}/repo"),
        );
        pipeline.inputs.insert(
            "logs_dir".to_string(),
            complex_input("${{ inputs.repo_dir }}/logs"),
        );
        pipeline
            .env
            .insert("ROOT".to_string(), "${{ bld_project_dir }}".to_string());

        let rctx = resolve(&pipeline, vec![]).unwrap();

        assert_eq!(rctx.get_env("ROOT").unwrap(), "/home/user/project");
        assert_eq!(
            rctx.get_input("repo_dir").unwrap(),
            "/home/user/project/repo"
        );
        assert_eq!(
            rctx.get_input("logs_dir").unwrap(),
            "/home/user/project/repo/logs"
        );
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
    pub fn cyclic_input_defaults_resolve_failure() {
        let mut pipeline = Pipeline::default();
        pipeline
            .inputs
            .insert("first".to_string(), complex_input("${{ inputs.second }}"));
        pipeline
            .inputs
            .insert("second".to_string(), complex_input("${{ inputs.first }}"));

        let error = resolve(&pipeline, vec![]).unwrap_err().to_string();

        assert!(error.contains("inputs.first, inputs.second"), "{error}");
        assert!(error.contains("cyclic"), "{error}");
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

            let error = resolve(&pipeline, vec![]).unwrap_err().to_string();

            assert!(
                error.contains("is not available at the start of a run"),
                "{error}"
            );
        }
    }
}
