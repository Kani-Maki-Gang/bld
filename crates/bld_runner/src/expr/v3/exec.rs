use crate::expr::v3::parser::{ExprParser, Rule};

use super::traits::{
    EvalExpr, EvalObject, ExprValue, ReadonlyRuntimeExprContext, WritableRuntimeExprContext,
};
use anyhow::{Result, anyhow, bail};
use pest::{Parser, iterators::Pair};

pub struct CommonExprExecutor<
    'a,
    T: EvalObject<'a>,
    RCtx: ReadonlyRuntimeExprContext<'a>,
    WCtx: WritableRuntimeExprContext,
> {
    obj_executor: &'a T,
    rctx: &'a RCtx,
    wctx: &'a WCtx,
}

impl<'a, T: EvalObject<'a>, RCtx: ReadonlyRuntimeExprContext<'a>, WCtx: WritableRuntimeExprContext>
    CommonExprExecutor<'a, T, RCtx, WCtx>
{
    pub fn new(obj_executor: &'a T, rctx: &'a RCtx, wctx: &'a mut WCtx) -> Self {
        Self {
            obj_executor,
            rctx,
            wctx,
        }
    }
}

impl<'a, T: EvalObject<'a>, RCtx: ReadonlyRuntimeExprContext<'a>, WCtx: WritableRuntimeExprContext>
    EvalExpr<'a> for CommonExprExecutor<'a, T, RCtx, WCtx>
{
    fn eval_cmp(&'a self, expr: Pair<'_, Rule>) -> Result<ExprValue<'a>> {
        if !matches!(
            expr.as_rule(),
            Rule::Equals | Rule::Greater | Rule::GreaterEquals | Rule::Less | Rule::LessEquals
        ) {
            bail!("expected comparison rule, found {:?}", expr.as_rule());
        }

        let mut equals = expr.into_inner();

        let left_expr = equals
            .next()
            .ok_or_else(|| anyhow!("no left operand found for equals expression"))?;
        let left = self.eval_symbol(left_expr)?;

        let Some(operator) = equals.next() else {
            bail!("expected comparison operator");
        };

        let right_expr = equals
            .next()
            .ok_or_else(|| anyhow!("no right operand found for equals expression"))?;
        let right = self.eval_symbol(right_expr)?;

        let operator_rule = operator.as_rule();
        match &operator_rule {
            Rule::EqualsOperator => left.try_eq(&right),

            Rule::GreaterOperator => left.try_ord(&right),

            Rule::GreaterEqualsOperator => left.try_ord(&right).and_then(|v| {
                if matches!(v, ExprValue::Boolean(false)) {
                    left.try_eq(&right)
                } else {
                    Ok(v)
                }
            }),

            Rule::LessOperator => right.try_ord(&left),

            Rule::LessEqualsOperator => right.try_ord(&left).and_then(|v| {
                if matches!(v, ExprValue::Boolean(false)) {
                    left.try_eq(&right)
                } else {
                    Ok(v)
                }
            }),

            _ => bail!("unexpected rule: {:?}", &operator_rule),
        }
    }

    fn eval_symbol(&'a self, expr: Pair<'_, Rule>) -> Result<ExprValue<'a>> {
        let Rule::Symbol = expr.as_rule() else {
            bail!("expected symbol rule, found {:?}", expr.as_rule());
        };

        let mut symbol = expr.into_inner().peekable();
        let peeked_symbol = symbol
            .peek()
            .ok_or_else(|| anyhow!("no symbol found in expression"))?;
        let symbol_span = peeked_symbol.as_span();
        let symbol_rule = peeked_symbol.as_rule();

        match &symbol_rule {
            Rule::Boolean | Rule::Number | Rule::String => symbol_span.as_str().try_into(),
            Rule::Object => self
                .obj_executor
                .eval_object(&mut symbol, self.rctx, self.wctx),
            _ => bail!("unexpected rule: {:?}", &symbol_rule),
        }
    }

    fn eval_expr(&'a self, expr: Pair<'_, Rule>) -> Result<ExprValue<'a>> {
        let Rule::Expression = expr.as_rule() else {
            bail!("expected expression rule, found {:?}", expr.as_rule());
        };

        let expr_inner = expr
            .into_inner()
            .next()
            .ok_or_else(|| anyhow!("no expression found"))?;

        let Rule::ExpressionInner = expr_inner.as_rule() else {
            bail!(
                "expected expression inner rule, found {:?}",
                expr_inner.as_rule()
            );
        };

        let actual_expr = expr_inner
            .into_inner()
            .next()
            .ok_or_else(|| anyhow!("no expression found"))?;

        match actual_expr.as_rule() {
            Rule::Equals | Rule::Greater | Rule::GreaterEquals | Rule::Less | Rule::LessEquals => {
                self.eval_cmp(actual_expr)
            }
            Rule::Symbol => self.eval_symbol(actual_expr),
            _ => bail!("unexpected rule: {:?}", actual_expr.as_rule()),
        }
    }

    fn eval_logical_expr(&'a self, expr: Pair<'_, Rule>) -> Result<ExprValue<'a>> {
        let Rule::LogicalExpression = expr.as_rule() else {
            bail!(
                "expected logical expression rule, found {:?}",
                expr.as_rule()
            );
        };

        let expr_inner = expr.into_inner();
        let mut accumulator: Option<ExprValue<'a>> = None;
        let mut current_value: Option<ExprValue<'a>> = None;

        for inner in expr_inner {
            match inner.as_rule() {
                Rule::Expression if current_value.is_none() => {
                    current_value = Some(self.eval_expr(inner)?);
                }

                Rule::Expression if current_value.is_some() => {
                    bail!(
                        "multiple expressions found in logical expression without any logical operator"
                    );
                }

                Rule::AndOperator => {
                    let Some(curr) = current_value.take() else {
                        bail!("no current value found before AND operator");
                    };

                    if let Some(acc) = accumulator {
                        accumulator = Some(acc.try_and(&curr)?);
                    } else {
                        accumulator = Some(curr);
                    }
                }

                Rule::OrOperator => {
                    let Some(curr) = current_value.take() else {
                        bail!("no current value found before OR operator");
                    };

                    if let Some(acc) = accumulator {
                        accumulator = Some(acc.try_or(&curr)?);
                    } else {
                        accumulator = Some(curr);
                    }
                }

                _ => bail!(
                    "unexpected rule in logical expression: {:?}",
                    inner.as_rule()
                ),
            }
        }

        accumulator.ok_or_else(|| anyhow!("no accumulator found in logical expression"))
    }

    fn eval(&'a self, expr: &'a str) -> Result<ExprValue<'a>> {
        let mut pairs = ExprParser::parse(Rule::Full, expr)?;
        let pair = pairs.next().ok_or_else(|| anyhow!("no expression found"))?;

        let inner = pair
            .into_inner()
            .next()
            .ok_or_else(|| anyhow!("no expression found"))?;

        match inner.as_rule() {
            Rule::LogicalExpression => self.eval_logical_expr(inner),

            Rule::Expression => self.eval_expr(inner),

            _ => bail!("unexpected rule: {:?}", inner.as_rule()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{iter::Peekable, collections::HashMap};

    use anyhow::anyhow;
    use pest::iterators::Pairs;

    use crate::expr::v3::traits::ExprText;

    use super::*;

    struct MockObjectExecutor;

    impl<'a> EvalObject<'a> for MockObjectExecutor {
        fn eval_object<RCtx: ReadonlyRuntimeExprContext<'a>, WCtx: WritableRuntimeExprContext>(
            &'a self,
            path: &mut Peekable<Pairs<'_, Rule>>,
            rctx: &RCtx,
            _wctx: &WCtx,
        ) -> Result<ExprValue<'a>> {
            let Some(object) = path.next() else {
                bail!("no object path present");
            };

            let Rule::Object = object.as_rule() else {
                bail!("expected object path");
            };

            let mut object_parts = object.into_inner();
            let Some(part) = object_parts.next() else {
                bail!("expected at least one part in the object path");
            };

            match part.as_span().as_str() {
                "inputs" => {
                    let Some(part) = object_parts.next() else {
                        bail!("expected name of input in object path");
                    };
                    let name = part.as_span().as_str();
                    rctx.get_input(name).map(|x| ExprValue::Text(ExprText::Ref(x)))
                }

                "env" => {
                    let Some(part) = object_parts.next() else {
                        bail!("expected name of input in object path");
                    };
                    let name = part.as_span().as_str();
                    let value = rctx.get_env(name);
                    value.map(|x| ExprValue::Text(ExprText::Ref(x)))
                }

                _ => unimplemented!(),
            }
        }
    }

    #[derive(Default)]
    struct MockReadonlyRuntimeExprContext {
        env: HashMap<String, String>,
        inputs: HashMap<String, String>,
        run_id: String,
        root_dir: String,
        project_dir: String,
        run_start_time: String
    }

    impl ReadonlyRuntimeExprContext<'_> for MockReadonlyRuntimeExprContext {
        fn get_env(&'_ self, name: &'_ str) -> Result<&'_ str> {
            self.env.get(name).map(|x| x.as_str()).ok_or_else(|| anyhow!("unable to find value"))
        }

        fn get_input(&'_ self, name: &'_ str) -> Result<&'_ str> {
            self.inputs.get(name).map(|x| x.as_str()).ok_or_else(|| anyhow!("unable to find value"))
        }

        fn get_run_id(&'_ self) -> &'_ str {
            &self.run_id
        }

        fn get_root_dir(&'_ self) -> &'_ str {
            &self.root_dir
        }

        fn get_project_dir(&'_ self) -> &'_ str {
            &self.project_dir
        }

        fn get_run_start_time(&'_ self) -> &'_ str {
            &self.run_start_time
        }
    }

    struct MockWritableRuntimeExprContext;

    impl WritableRuntimeExprContext for MockWritableRuntimeExprContext {
        fn get_output(&self, _name: &str) -> Result<&str> {
            unimplemented!();
        }

        fn set_output(&mut self, _name: String, _value: String) -> Result<()> {
            unimplemented!();
        }
    }

    #[test]
    pub fn number_eval_success() {
        let data = vec![
            ("${{ 100 }}", ExprValue::Number(100.0)),
            ("${{ 100.0 }}", ExprValue::Number(100.0)),
            ("${{ 150.20 }}", ExprValue::Number(150.20)),
            ("${{ 0.0 }}", ExprValue::Number(0.0)),
            ("${{ 0 }}", ExprValue::Number(0.0)),
            ("${{ -100 }}", ExprValue::Number(-100.0)),
            ("${{ -100.0 }}", ExprValue::Number(-100.0)),
            ("${{ -150.20 }}", ExprValue::Number(-150.20)),
        ];

        let mut wctx = MockWritableRuntimeExprContext;
        let rctx = MockReadonlyRuntimeExprContext::default();
        let exec = CommonExprExecutor::new(
            &MockObjectExecutor,
            &rctx,
            &mut wctx,
        );

        for (expr, expected) in data {
            let Ok(value) = exec.eval(expr) else {
                panic!("failed to parse expression: {expr}");
            };

            let ExprValue::Number(value) = value else {
                panic!("expected number, found {:?}", value);
            };

            let ExprValue::Number(expected) = expected else {
                panic!("expected number, found {:?}", expected);
            };

            assert_eq!(value, expected);
        }
    }

    #[test]
    pub fn boolean_eval_success() {
        let data = vec![
            ("${{ true }}", ExprValue::Boolean(true)),
            ("${{ false }}", ExprValue::Boolean(false)),
        ];

        let mut wctx = MockWritableRuntimeExprContext;
        let rctx = MockReadonlyRuntimeExprContext::default();
        let exec = CommonExprExecutor::new(
            &MockObjectExecutor,
            &rctx,
            &mut wctx,
        );

        for (expr, expected) in data {
            let Ok(value) = exec.eval(expr) else {
                panic!("failed to parse expression: {expr}");
            };

            let ExprValue::Boolean(value) = value else {
                panic!("expected boolean, found {:?}", value);
            };

            let ExprValue::Boolean(expected) = expected else {
                panic!("expected boolean, found {:?}", expected);
            };

            assert_eq!(value, expected);
        }
    }

    #[test]
    pub fn string_eval_success() {
        let data = vec![
            ("${{ \"hello\" }}", ExprValue::Text(ExprText::Owned("hello".to_string()))),
        ];

        let mut wctx = MockWritableRuntimeExprContext;
        let rctx = MockReadonlyRuntimeExprContext::default();
        let exec = CommonExprExecutor::new(
            &MockObjectExecutor,
            &rctx,
            &mut wctx,
        );

        for (expr, expected) in data {
            match exec.eval(expr) {
                Ok(value) => {
                    let ExprValue::Text(value) = value else {
                        panic!("expected text, found {:?}", value);
                    };

                    let ExprValue::Text(expected) = expected else {
                        panic!("expected text, found {:?}", expected);
                    };

                    assert_eq!(value, expected);
                }
                Err(e) => {
                    panic!("failed to parse expression {expr} due to {e}");
                }
            }
        }
    }

    #[test]
    pub fn object_eval_succes() {
        let mut wctx = MockWritableRuntimeExprContext;
        let mut rctx = MockReadonlyRuntimeExprContext::default();
        rctx.inputs.insert("name".to_string(), "John".to_string());
        rctx.inputs.insert("surname".to_string(), "Doe".to_string());
        rctx.inputs.insert("age".to_string(), "32".to_string());
        rctx.env.insert("WORKDIR".to_string(), "/home/somedir".to_string());
        rctx.env.insert("NODE".to_string(), "lts".to_string());

        let data = vec![
            ("${{ inputs.name }}", ExprValue::Text(ExprText::Ref("John"))),
            ("${{ inputs.surname }}", ExprValue::Text(ExprText::Ref("Doe"))),
            ("${{ inputs.age }}", ExprValue::Number(32.0)),
            ("${{ env.WORKDIR }}", ExprValue::Text(ExprText::Ref("/home/somedir"))),
            ("${{ env.NODE }}", ExprValue::Text(ExprText::Ref("lts"))),
        ];

        let exec = CommonExprExecutor::new(
            &MockObjectExecutor,
            &rctx,
            &mut wctx,
        );

        for (expr, expected) in data {
            match exec.eval(expr) {
                Ok(value) => {
                    let ExprValue::Text(value) = value else {
                        panic!("expected text, found {:?}", value);
                    };

                    let ExprValue::Text(expected) = expected else {
                        panic!("expected text, found {:?}", expected);
                    };

                    assert_eq!(value, expected);
                }
                Err(e) => {
                    panic!("failed to parse expression {expr} due to {e}");
                }
            }
        }
    }
}
