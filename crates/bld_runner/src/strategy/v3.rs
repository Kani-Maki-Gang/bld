use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[cfg(feature = "all")]
use {
    crate::{
        expr::v3::{
            exec::CommonExprExecutor,
            traits::{
                EvalExpr, EvalObject, ExprValue, ReadonlyRuntimeExprContext,
                WritableRuntimeExprContext,
            },
        },
        validator::v3::{Validate, ValidatorContext},
    },
    anyhow::{Result, bail},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MatrixValue {
    Array(Vec<String>),
    Expr(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FailFastValue {
    Bool(bool),
    Expr(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strategy {
    #[serde(default)]
    pub matrix: HashMap<String, MatrixValue>,
    pub fail_fast: Option<FailFastValue>,
}

impl Strategy {
    pub fn default_fail_fast() -> bool {
        true
    }

    pub fn matrix_keys(&self) -> HashSet<&str> {
        self.matrix.keys().map(|k| k.as_str()).collect()
    }
}

#[cfg(feature = "all")]
impl Strategy {
    pub fn combinations<'a, T, RCtx, WCtx>(
        &'a self,
        exec: &CommonExprExecutor<'a, T, RCtx, WCtx>,
    ) -> Result<Vec<HashMap<String, String>>>
    where
        T: EvalObject<'a>,
        RCtx: ReadonlyRuntimeExprContext<'a>,
        WCtx: WritableRuntimeExprContext,
    {
        let mut keys: Vec<&String> = self.matrix.keys().collect();
        keys.sort();

        let mut combinations: Vec<HashMap<String, String>> = vec![HashMap::new()];

        for key in keys {
            let value = self
                .matrix
                .get(key)
                .ok_or_else(|| anyhow::anyhow!("matrix key '{key}' not found"))?;

            let values: Vec<String> = match value {
                MatrixValue::Array(items) => items.clone(),
                MatrixValue::Expr(expr) => {
                    let result = exec.eval(expr)?;
                    let ExprValue::Array(items) = result else {
                        bail!(
                            "matrix key '{key}' must evaluate to an array, found {}",
                            result.type_as_string()
                        );
                    };
                    items.iter().map(|x| x.to_string()).collect()
                }
            };

            if values.is_empty() {
                bail!("matrix key '{key}' has no values defined");
            }

            let mut next = Vec::with_capacity(combinations.len() * values.len());
            for combination in &combinations {
                for value in &values {
                    let mut next_combination = combination.clone();
                    next_combination.insert(key.clone(), value.clone());
                    next.push(next_combination);
                }
            }
            combinations = next;
        }

        Ok(combinations)
    }

    pub fn resolve_fail_fast<'a, T, RCtx, WCtx>(
        &'a self,
        exec: &CommonExprExecutor<'a, T, RCtx, WCtx>,
    ) -> Result<bool>
    where
        T: EvalObject<'a>,
        RCtx: ReadonlyRuntimeExprContext<'a>,
        WCtx: WritableRuntimeExprContext,
    {
        match self.fail_fast.as_ref() {
            None => Ok(Self::default_fail_fast()),
            Some(FailFastValue::Bool(value)) => Ok(*value),
            Some(FailFastValue::Expr(expr)) => match exec.eval(expr)? {
                ExprValue::Boolean(value) => Ok(value),
                other => bail!(
                    "fail_fast must evaluate to a boolean, found {}",
                    other.type_as_string()
                ),
            },
        }
    }
}

#[cfg(feature = "all")]
impl<'a> Validate<'a> for Strategy {
    async fn validate<C: ValidatorContext<'a>>(&'a self, ctx: &mut C) {
        ctx.push_section("matrix");
        if self.matrix.is_empty() {
            ctx.append_error("Strategy matrix must define at least one key");
        }

        for (key, value) in self.matrix.iter() {
            ctx.push_section(key);
            match value {
                MatrixValue::Array(items) => {
                    if items.is_empty() {
                        ctx.append_error("Matrix key must have at least one value defined");
                    }
                }
                MatrixValue::Expr(expr) => {
                    if ctx.expression_count(expr) > 1 {
                        ctx.append_error("Matrix value must contain at most one expression");
                    } else {
                        ctx.validate_array_expression(expr);
                    }
                }
            }
            ctx.pop_section();
        }
        ctx.pop_section();

        if let Some(fail_fast) = self.fail_fast.as_ref() {
            ctx.push_section("fail_fast");
            if let FailFastValue::Expr(expr) = fail_fast {
                if ctx.expression_count(expr) > 1 {
                    ctx.append_error("fail_fast must contain at most one expression");
                } else {
                    ctx.validate_expressions(expr);
                }
            }
            ctx.pop_section();
        }
    }
}

#[cfg(feature = "all")]
pub fn validate_matrix_refs<'a, C: ValidatorContext<'a>>(
    ctx: &mut C,
    value: &str,
    available: &HashSet<&str>,
) {
    for name in ctx.matrix_refs(value) {
        if !available.contains(name.as_str()) {
            ctx.append_error(&format!("matrix key '{name}' is not defined"));
        }
    }
}

#[cfg(all(test, feature = "all"))]
mod tests {
    use super::*;

    #[test]
    pub fn matrix_value_array_serde_roundtrip() {
        let yaml = "os:\n  - linux\n  - windows\n";
        let value: HashMap<String, MatrixValue> = serde_yaml_ng::from_str(yaml).unwrap();
        let os = value.get("os").unwrap();
        assert!(
            matches!(os, MatrixValue::Array(items) if items == &vec!["linux".to_string(), "windows".to_string()])
        );
    }

    #[test]
    pub fn matrix_value_expr_serde_roundtrip() {
        let yaml = "os: ${{ inputs.oses }}\n";
        let value: HashMap<String, MatrixValue> = serde_yaml_ng::from_str(yaml).unwrap();
        let os = value.get("os").unwrap();
        assert!(matches!(os, MatrixValue::Expr(expr) if expr == "${{ inputs.oses }}"));
    }

    #[test]
    pub fn fail_fast_value_serde_roundtrip() {
        let yaml_bool = "true";
        let value: FailFastValue = serde_yaml_ng::from_str(yaml_bool).unwrap();
        assert!(matches!(value, FailFastValue::Bool(true)));

        let yaml_expr = "${{ inputs.fail_fast }}";
        let value: FailFastValue = serde_yaml_ng::from_str(yaml_expr).unwrap();
        assert!(matches!(value, FailFastValue::Expr(expr) if expr == "${{ inputs.fail_fast }}"));
    }

    #[test]
    pub fn strategy_serde_roundtrip() {
        let yaml = r#"
matrix:
  os:
    - linux
    - windows
  version:
    - v2
    - v3
fail_fast: false
"#;
        let strategy: Strategy = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(strategy.matrix.len(), 2);
        assert!(matches!(
            strategy.fail_fast,
            Some(FailFastValue::Bool(false))
        ));
    }

    #[test]
    pub fn strategy_default_fail_fast() {
        let yaml = r#"
matrix:
  os:
    - linux
"#;
        let strategy: Strategy = serde_yaml_ng::from_str(yaml).unwrap();
        assert!(strategy.fail_fast.is_none());
        assert!(Strategy::default_fail_fast());
    }
}

#[cfg(all(test, feature = "all"))]
mod exec_tests {
    use std::collections::HashMap;

    use crate::{
        expr::v3::{
            context::CommonReadonlyRuntimeExprContext, exec::CommonExprExecutor,
            traits::MockWritableRuntimeExprContext,
        },
        inputs::v3::Input,
        pipeline::v3::Pipeline,
        strategy::v3::{FailFastValue, MatrixValue, Strategy},
    };

    #[test]
    pub fn combinations_literal_array_success() {
        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        let mut matrix = HashMap::new();
        matrix.insert(
            "os".to_string(),
            MatrixValue::Array(vec!["linux".to_string(), "windows".to_string()]),
        );
        matrix.insert(
            "version".to_string(),
            MatrixValue::Array(vec!["v2".to_string(), "v3".to_string()]),
        );
        let strategy = Strategy {
            matrix,
            fail_fast: None,
        };

        let combinations = strategy.combinations(&exec).unwrap();
        assert_eq!(combinations.len(), 4);

        for combination in &combinations {
            assert!(combination.contains_key("os"));
            assert!(combination.contains_key("version"));
        }
    }

    #[test]
    pub fn combinations_expr_array_success() {
        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        let mut matrix = HashMap::new();
        matrix.insert(
            "os".to_string(),
            MatrixValue::Expr("${{ [\"linux\", \"windows\"] }}".to_string()),
        );
        let strategy = Strategy {
            matrix,
            fail_fast: None,
        };

        let combinations = strategy.combinations(&exec).unwrap();
        assert_eq!(combinations.len(), 2);
    }

    #[test]
    pub fn combinations_expr_non_array_failure() {
        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let mut pipeline = Pipeline::default();
        pipeline
            .inputs
            .insert("oses".to_string(), Input::Simple("linux".to_string()));
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        let mut matrix = HashMap::new();
        matrix.insert(
            "os".to_string(),
            MatrixValue::Expr("${{ inputs.oses }}".to_string()),
        );
        let strategy = Strategy {
            matrix,
            fail_fast: None,
        };

        assert!(strategy.combinations(&exec).is_err());
    }

    #[test]
    pub fn resolve_fail_fast_default_success() {
        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        let strategy = Strategy {
            matrix: HashMap::new(),
            fail_fast: None,
        };

        assert!(strategy.resolve_fail_fast(&exec).unwrap());
    }

    #[test]
    pub fn resolve_fail_fast_bool_success() {
        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        let strategy = Strategy {
            matrix: HashMap::new(),
            fail_fast: Some(FailFastValue::Bool(false)),
        };

        assert!(!strategy.resolve_fail_fast(&exec).unwrap());
    }

    #[test]
    pub fn resolve_fail_fast_expr_success() {
        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        let strategy = Strategy {
            matrix: HashMap::new(),
            fail_fast: Some(FailFastValue::Expr("${{ false }}".to_string())),
        };

        assert!(!strategy.resolve_fail_fast(&exec).unwrap());
    }
}
