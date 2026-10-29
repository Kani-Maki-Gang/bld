use crate::expr::v3::exec::{CommonExprExecutor, eval_all_expressions};
use crate::expr::v3::parser;
use crate::expr::v3::traits::{EvalObject, ExprText, ExprType};
use crate::inputs::v3::{Input, InputValue, defaults_in_order};
use crate::scalar::{ScalarType, ScalarValue};

use super::traits::{
    EvalExpr, ExprValue, OutputScope, ReadonlyRuntimeExprContext, WritableRuntimeExprContext,
};
use anyhow::{Context, Error, Result, anyhow, bail};
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

    fn get_matrix_value<'a>(&'a self, name: &str) -> Result<ExprValue<'a>> {
        Err(out_of_scope(&format!("matrix.{name}")))
    }
}

#[derive(Clone, Debug, Default)]
pub struct CommonReadonlyRuntimeExprContext {
    pub config: Arc<BldConfig>,
    pub inputs: Arc<HashMap<String, ExprValue<'static>>>,
    pub env: Arc<HashMap<String, String>>,
    pub run_id: String,
    pub run_start_time: String,
}

impl CommonReadonlyRuntimeExprContext {
    pub fn new(
        config: Arc<BldConfig>,
        inputs: Arc<HashMap<String, ExprValue<'static>>>,
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

    pub fn clone_with(&self, closure: impl FnOnce(&mut Self)) -> Self {
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

    fn get_input(&'a self, name: &'a str) -> Result<ExprValue<'a>> {
        self.inputs
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow!("input '{name}' not found"))
    }

    fn get_env(&'a self, name: &'a str) -> Result<ExprValue<'a>> {
        self.env
            .get(name)
            .map(|x| ExprValue::Text(ExprText::Ref(x.as_str())))
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

#[cfg(test)]
pub fn text_inputs(values: &HashMap<String, String>) -> Arc<HashMap<String, ExprValue<'static>>> {
    values
        .iter()
        .map(|(k, v)| (k.clone(), ExprValue::Text(ExprText::Owned(v.clone()))))
        .collect::<HashMap<_, _>>()
        .into_arc()
}

pub fn input_from_text(name: &str, ty: ScalarType, raw: &str) -> Result<ExprValue<'static>> {
    let value = match ty {
        ScalarType::Text => Some(ExprValue::Text(ExprText::Owned(raw.to_string()))),
        ScalarType::Number => raw
            .parse::<f64>()
            .ok()
            .filter(|x| x.is_finite())
            .map(|value| ExprValue::Number {
                value,
                raw: ExprText::Owned(raw.to_string()),
            }),
        ScalarType::Boolean => raw.parse::<bool>().ok().map(ExprValue::Boolean),
        ScalarType::Array => match ExprValue::try_from(raw) {
            Ok(value @ ExprValue::Array(_)) => Some(value),
            _ => None,
        },
    };
    value.ok_or_else(|| anyhow!("input '{name}' must be a {}, found '{raw}'", ty.as_str()))
}

fn zero_value(ty: ScalarType) -> ExprValue<'static> {
    match ty {
        ScalarType::Text => ExprValue::Text(ExprText::Owned(String::new())),
        ScalarType::Number => ExprValue::from(&ScalarValue::Number(0.0)),
        ScalarType::Boolean => ExprValue::Boolean(false),
        ScalarType::Array => ExprValue::Array(Vec::new()),
    }
}

fn input_from_default<'a, E: EvalExpr<'a>>(
    exec: &E,
    regex: &regex::Regex,
    name: &str,
    input: &Input,
    default: &'a InputValue,
) -> Result<ExprValue<'static>> {
    let value = match default {
        InputValue::Array(items) => ExprValue::Array(items.iter().map(ExprValue::from).collect()),
        InputValue::Scalar(ScalarValue::Text(raw))
            if input.scalar_type != ScalarType::Text && regex.is_match(raw) =>
        {
            exec.eval(raw.trim())?.into_owned()
        }
        InputValue::Scalar(ScalarValue::Text(raw)) => {
            ExprValue::Text(ExprText::Owned(eval_all_expressions(exec, regex, raw)?))
        }
        InputValue::Scalar(value) => ExprValue::from(value),
    };

    let expected = ExprType::from(input.scalar_type);
    if !value.expr_type().is_some_and(|x| x.matches(&expected)) {
        bail!(
            "input '{name}' must be a {expected}, found {}",
            value.type_as_string()
        );
    }
    Ok(value)
}

pub fn expr_rctx<'a, T: for<'x> EvalObject<'x>>(
    options: CommonReadonlyRuntimeExprContextOptions<'a, T>,
) -> Result<CommonReadonlyRuntimeExprContext> {
    let regex = parser::new_regex()?;
    let mut env = (*options.env).clone();

    // Supplied values always take precedence over the ones declared in the file.
    let mut inputs = HashMap::new();
    for (name, raw) in options.inputs.iter() {
        let value = match options.declared_inputs.get(name) {
            Some(input) => input_from_text(name, input.scalar_type, raw)?,
            None => ExprValue::Text(ExprText::Owned(raw.to_string())),
        };
        inputs.insert(name.to_string(), value);
    }

    let defaults = defaults_in_order(
        options
            .declared_inputs
            .iter()
            .filter(|(name, _)| !inputs.contains_key(*name)),
    )
    .context("unable to resolve inputs at the start of the run")?;
    for (name, input) in defaults {
        let value = match input.default_value() {
            Some(default) => {
                let rctx = CommonReadonlyRuntimeExprContext::new(
                    options.config.clone(),
                    inputs.clone().into_arc(),
                    options.env.clone(),
                    options.run_id.clone(),
                    options.run_start_time.clone(),
                );
                let exec = CommonExprExecutor::new(options.obj, &rctx, &START_OF_RUN_WCTX);
                input_from_default(&exec, &regex, name, input, default).with_context(|| {
                    format!("unable to resolve inputs.{name} at the start of the run")
                })?
            }
            None if input.is_required() => continue,
            None => zero_value(input.scalar_type),
        };
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
        Input::text(default)
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
            rctx.get_input("worktree_root").unwrap().to_string(),
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
            rctx.get_input("repo_dir").unwrap().to_string(),
            "/home/user/project/repo"
        );
        assert_eq!(
            rctx.get_env("LOGS").unwrap().to_string(),
            "/home/user/project/repo/logs"
        );
    }

    #[test]
    pub fn input_default_referencing_another_default_resolve_success() {
        let mut pipeline = Pipeline::default();
        pipeline.inputs.insert(
            "third".to_string(),
            complex_input("${{ inputs.second }}/leaf"),
        );
        pipeline
            .inputs
            .insert("first".to_string(), complex_input("/root"));
        pipeline.inputs.insert(
            "second".to_string(),
            complex_input("${{ inputs.first }}/sub"),
        );

        let rctx = resolve(&pipeline, vec![]).unwrap();

        assert_eq!(rctx.get_input("second").unwrap().to_string(), "/root/sub");
        assert_eq!(
            rctx.get_input("third").unwrap().to_string(),
            "/root/sub/leaf"
        );
    }

    #[test]
    pub fn input_default_referencing_supplied_input_resolve_success() {
        let mut pipeline = Pipeline::default();
        pipeline
            .inputs
            .insert("first".to_string(), complex_input("/root"));
        pipeline.inputs.insert(
            "second".to_string(),
            complex_input("${{ inputs.first }}/sub"),
        );

        let rctx = resolve(&pipeline, vec![("first", "/home")]).unwrap();

        assert_eq!(rctx.get_input("second").unwrap().to_string(), "/home/sub");
    }

    #[test]
    pub fn typed_input_default_referencing_another_default_resolve_success() {
        let mut pipeline = Pipeline::default();
        pipeline.inputs.insert(
            "count".to_string(),
            Input::typed(
                ScalarType::Number,
                Some(InputValue::Scalar(ScalarValue::Number(3.0))),
            ),
        );
        pipeline.inputs.insert(
            "more".to_string(),
            Input::typed(
                ScalarType::Boolean,
                Some(InputValue::Scalar(ScalarValue::Text(
                    "${{ inputs.count > 2 }}".to_string(),
                ))),
            ),
        );

        let rctx = resolve(&pipeline, vec![]).unwrap();

        assert_eq!(rctx.get_input("more").unwrap(), ExprValue::Boolean(true));
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

        let error = format!("{:#}", resolve(&pipeline, vec![]).unwrap_err());

        assert!(
            error.contains("input defaults contain a cyclic reference between: first, second"),
            "{error}"
        );
    }

    #[test]
    pub fn cyclic_input_defaults_with_supplied_input_resolve_success() {
        let mut pipeline = Pipeline::default();
        pipeline
            .inputs
            .insert("first".to_string(), complex_input("${{ inputs.second }}"));
        pipeline
            .inputs
            .insert("second".to_string(), complex_input("${{ inputs.first }}"));

        let rctx = resolve(&pipeline, vec![("first", "value")]).unwrap();

        assert_eq!(rctx.get_input("second").unwrap().to_string(), "value");
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

        assert_eq!(rctx.get_input("image").unwrap().to_string(), "ubuntu:22.04");
    }

    #[test]
    pub fn input_without_default_is_left_unresolved() {
        let mut pipeline = Pipeline::default();
        pipeline.inputs.insert(
            "image".to_string(),
            Input {
                required: true,
                ..Input::typed(ScalarType::Text, None)
            },
        );

        let rctx = resolve(&pipeline, vec![]).unwrap();

        assert!(rctx.get_input("image").is_err());
    }

    #[test]
    pub fn optional_input_without_default_resolves_to_its_type_default() {
        let mut pipeline = Pipeline::default();
        for (name, ty) in [
            ("tag", ScalarType::Text),
            ("count", ScalarType::Number),
            ("deploy", ScalarType::Boolean),
            ("oses", ScalarType::Array),
        ] {
            pipeline
                .inputs
                .insert(name.to_string(), Input::typed(ty, None));
        }

        let rctx = resolve(&pipeline, vec![]).unwrap();

        assert_eq!(
            rctx.get_input("tag").unwrap(),
            ExprValue::Text(ExprText::Owned(String::new()))
        );
        assert_eq!(rctx.get_input("count").unwrap().to_string(), "0");
        assert_eq!(rctx.get_input("deploy").unwrap(), ExprValue::Boolean(false));
        assert_eq!(
            rctx.get_input("oses").unwrap(),
            ExprValue::Array(Vec::new())
        );
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

    #[test]
    pub fn supplied_inputs_are_converted_to_their_type_resolve_success() {
        let mut pipeline = Pipeline::default();
        for (name, ty) in [
            ("deploy", ScalarType::Boolean),
            ("count", ScalarType::Number),
            ("oses", ScalarType::Array),
            ("name", ScalarType::Text),
        ] {
            pipeline
                .inputs
                .insert(name.to_string(), Input::typed(ty, None));
        }

        let rctx = resolve(
            &pipeline,
            vec![
                ("deploy", "false"),
                ("count", "5"),
                ("oses", r#"["a","b"]"#),
                ("name", "5"),
                ("undeclared", "true"),
            ],
        )
        .unwrap();

        assert_eq!(rctx.get_input("deploy").unwrap(), ExprValue::Boolean(false));
        assert_eq!(
            rctx.get_input("count").unwrap().expr_type(),
            Some(ExprType::Number)
        );
        assert_eq!(
            rctx.get_input("oses").unwrap(),
            ExprValue::try_from(r#"["a","b"]"#).unwrap()
        );
        assert_eq!(
            rctx.get_input("name").unwrap(),
            ExprValue::Text(ExprText::Owned("5".to_string()))
        );
        assert_eq!(
            rctx.get_input("undeclared").unwrap(),
            ExprValue::Text(ExprText::Owned("true".to_string()))
        );
    }

    #[test]
    pub fn supplied_input_of_another_type_resolve_failure() {
        let data = [
            (ScalarType::Number, "abc"),
            (ScalarType::Boolean, "yes"),
            (ScalarType::Array, "linux"),
            (ScalarType::Array, r#"["a", 1]"#),
        ];

        for (ty, raw) in data {
            let mut pipeline = Pipeline::default();
            pipeline
                .inputs
                .insert("value".to_string(), Input::typed(ty, None));

            let error = format!(
                "{:#}",
                resolve(&pipeline, vec![("value", raw)]).unwrap_err()
            );
            assert!(
                error.contains(&format!("input 'value' must be a {}", ty.as_str())),
                "{error}"
            );
        }
    }

    #[test]
    pub fn typed_defaults_resolve_success() {
        let mut pipeline = Pipeline::default();
        pipeline.inputs.insert(
            "deploy".to_string(),
            Input::typed(
                ScalarType::Boolean,
                Some(InputValue::Scalar(ScalarValue::Boolean(true))),
            ),
        );
        pipeline.inputs.insert(
            "oses".to_string(),
            Input::typed(
                ScalarType::Array,
                Some(InputValue::Array(vec![ScalarValue::Text(
                    "linux".to_string(),
                )])),
            ),
        );
        pipeline.inputs.insert(
            "count".to_string(),
            Input::typed(
                ScalarType::Number,
                Some(InputValue::Scalar(ScalarValue::Text(
                    "${{ 3 }}".to_string(),
                ))),
            ),
        );

        let rctx = resolve(&pipeline, vec![]).unwrap();

        assert_eq!(rctx.get_input("deploy").unwrap(), ExprValue::Boolean(true));
        assert_eq!(
            rctx.get_input("oses").unwrap(),
            ExprValue::Array(vec![ExprValue::Text(ExprText::Owned("linux".to_string()))])
        );
        assert_eq!(
            rctx.get_input("count").unwrap().expr_type(),
            Some(ExprType::Number)
        );
    }

    #[test]
    pub fn default_expression_of_another_type_resolve_failure() {
        let mut pipeline = Pipeline::default();
        pipeline.inputs.insert(
            "count".to_string(),
            Input::typed(
                ScalarType::Number,
                Some(InputValue::Scalar(ScalarValue::Text(
                    "${{ bld_project_dir }}".to_string(),
                ))),
            ),
        );

        let error = format!("{:#}", resolve(&pipeline, vec![]).unwrap_err());
        assert!(
            error.contains("input 'count' must be a number, found text"),
            "{error}"
        );
    }
}
