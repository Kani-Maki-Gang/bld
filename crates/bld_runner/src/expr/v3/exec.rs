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
    fn eval_cmp(&'a self, expr: Pair<'a, Rule>) -> Result<ExprValue<'a>> {
        if !matches!(
            expr.as_rule(),
            Rule::Equals
                | Rule::NotEquals
                | Rule::Greater
                | Rule::GreaterEquals
                | Rule::Less
                | Rule::LessEquals
        ) {
            bail!("expected comparison rule, found {:?}", expr.as_rule());
        }

        let mut expr = expr.into_inner();

        let left_expr = expr
            .next()
            .ok_or_else(|| anyhow!("no left operand found for comparison expression"))?;
        let left = self.eval_symbol(left_expr)?;

        let Some(operator) = expr.next() else {
            bail!("expected comparison operator");
        };

        let right_expr = expr
            .next()
            .ok_or_else(|| anyhow!("no right operand found for comparison expression"))?;
        let right = self.eval_symbol(right_expr)?;

        let operator_rule = operator.as_rule();
        match &operator_rule {
            Rule::EqualsOperator => left.try_eq(&right),

            Rule::NotEqualsOperator => left.try_not_eq(&right),

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

    fn eval_symbol(&'a self, expr: Pair<'a, Rule>) -> Result<ExprValue<'a>> {
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

    fn eval_expr(&'a self, expr: Pair<'a, Rule>) -> Result<ExprValue<'a>> {
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
            Rule::Equals
            | Rule::NotEquals
            | Rule::Greater
            | Rule::GreaterEquals
            | Rule::Less
            | Rule::LessEquals => self.eval_cmp(actual_expr),
            Rule::Symbol => self.eval_symbol(actual_expr),
            _ => bail!("unexpected rule: {:?}", actual_expr.as_rule()),
        }
    }

    fn eval_logical_expr(&'a self, expr: Pair<'a, Rule>) -> Result<ExprValue<'a>> {
        let Rule::LogicalExpression = expr.as_rule() else {
            bail!(
                "expected logical expression rule, found {:?}",
                expr.as_rule()
            );
        };

        let expr_inner = expr.into_inner();
        let mut result: Option<ExprValue<'a>> = None;
        let mut operator: Option<Rule> = None;

        for inner in expr_inner {
            match inner.as_rule() {
                Rule::Expression => {
                    let value = self.eval_expr(inner)?;

                    // this is the case of starting the evaluation of the logical expression
                    // during the rest of the evaluation there should always be a result value and
                    // an operator.
                    let Some(operator) = operator else {
                        result = Some(value);
                        continue;
                    };

                    match operator {
                        Rule::AndOperator => {
                            if let Some(res) = result {
                                result = Some(res.try_and(&value)?);
                            }
                        }

                        Rule::OrOperator => {
                            if let Some(res) = result {
                                result = Some(res.try_or(&value)?);
                            }
                        }

                        _ => bail!(
                            "invalid operator encountered during evaluation of logical expression"
                        ),
                    }
                }

                Rule::AndOperator => {
                    operator = Some(Rule::AndOperator);
                }

                Rule::OrOperator => {
                    operator = Some(Rule::OrOperator);
                }

                _ => {
                    bail!("invalid expression encountered during evaluation of logical expression")
                }
            }
        }

        result.ok_or_else(|| anyhow!("no value was computed during logical expression evaluation"))
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
    use crate::{
        expr::v3::{
            context::CommonReadonlyRuntimeExprContext,
            traits::{ExprText, MockWritableRuntimeExprContext},
        },
        inputs::v3::Input,
        pipeline::v3::Pipeline,
    };
    use anyhow::Result;

    use super::*;

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

        let mut wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);

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

        let mut wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);

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
        let data = vec![(
            "${{ \"hello\" }}",
            ExprValue::Text(ExprText::Owned("hello".to_string())),
        )];

        let mut wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);

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
        let data = vec![
            ("${{ inputs.name }}", ExprValue::Text(ExprText::Ref("John"))),
            (
                "${{ inputs.surname }}",
                ExprValue::Text(ExprText::Ref("Doe")),
            ),
            ("${{ inputs.age }}", ExprValue::Text(ExprText::Ref("32"))),
            (
                "${{ env.WORKDIR }}",
                ExprValue::Text(ExprText::Ref("/home/somedir")),
            ),
            ("${{ env.NODE }}", ExprValue::Text(ExprText::Ref("lts"))),
        ];

        let mut wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();

        let mut pipeline = Pipeline::default();
        pipeline
            .inputs
            .insert("name".to_string(), Input::Simple("John".to_string()));
        pipeline
            .inputs
            .insert("surname".to_string(), Input::Simple("Doe".to_string()));
        pipeline
            .inputs
            .insert("age".to_string(), Input::Simple("32".to_string()));
        pipeline
            .env
            .insert("WORKDIR".to_string(), "/home/somedir".to_string());
        pipeline.env.insert("NODE".to_string(), "lts".to_string());

        let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);

        for (expr, expected) in data {
            match exec.eval(expr) {
                Ok(value) => {
                    assert!(matches!(
                        value.try_eq(&expected),
                        Result::Ok(ExprValue::Boolean(true))
                    ));
                }
                Err(e) => {
                    panic!("failed to parse expression {expr} due to {e}");
                }
            }
        }
    }

    #[test]
    pub fn equals_operator_eval_success() {
        let data: Vec<(&str, Result<ExprValue>)> = vec![
            ("${{ true == true }}", Ok(ExprValue::Boolean(true))),
            ("${{ true == false }}", Ok(ExprValue::Boolean(false))),
            ("${{ false == true }}", Ok(ExprValue::Boolean(false))),
            ("${{ false == false }}", Ok(ExprValue::Boolean(true))),
            ("${{ 4 == 4.0 }}", Ok(ExprValue::Boolean(true))),
            ("${{ 4 == 4 }}", Ok(ExprValue::Boolean(true))),
            ("${{ 4 == 5 }}", Ok(ExprValue::Boolean(false))),
            ("${{ 5 == 4 }}", Ok(ExprValue::Boolean(false))),
            (
                "${{ \"hello\" == \"hello\" }}",
                Ok(ExprValue::Boolean(true)),
            ),
            (
                "${{ \"hello\" == \"hello world\" }}",
                Ok(ExprValue::Boolean(false)),
            ),
            ("${{ 4 == true }}", Err(anyhow!(""))),
            ("${{ false == 52.0 }}", Err(anyhow!(""))),
            ("${{ \"hello\" == 52.0 }}", Err(anyhow!(""))),
        ];

        let mut wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);

        for (expr, expected) in data {
            let value = exec.eval(expr);

            if let Ok(expected) = expected {
                let Ok(value) = value else {
                    panic!("invalid result after eval");
                };
                assert!(matches!(
                    value.try_eq(&expected),
                    Ok(ExprValue::Boolean(true))
                ));
                continue;
            }

            if expected.is_err() && value.is_ok() {
                panic!("invalid result after eval");
            }
        }
    }

    #[test]
    pub fn not_equals_operator_eval_success() {
        let data: Vec<(&str, Result<ExprValue>)> = vec![
            ("${{ true != true }}", Ok(ExprValue::Boolean(false))),
            ("${{ true != false }}", Ok(ExprValue::Boolean(true))),
            ("${{ false != true }}", Ok(ExprValue::Boolean(true))),
            ("${{ false != false }}", Ok(ExprValue::Boolean(false))),
            ("${{ 4 != 4.0 }}", Ok(ExprValue::Boolean(false))),
            ("${{ 4 != 4 }}", Ok(ExprValue::Boolean(false))),
            ("${{ 4 != 5 }}", Ok(ExprValue::Boolean(true))),
            ("${{ 5 != 4 }}", Ok(ExprValue::Boolean(true))),
            (
                "${{ \"hello\" != \"hello\" }}",
                Ok(ExprValue::Boolean(false)),
            ),
            (
                "${{ \"hello\" != \"hello world\" }}",
                Ok(ExprValue::Boolean(true)),
            ),
            ("${{ 4 != true }}", Err(anyhow!(""))),
            ("${{ false != 52.0 }}", Err(anyhow!(""))),
            ("${{ \"hello\" != 52.0 }}", Err(anyhow!(""))),
        ];

        let mut wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);

        for (expr, expected) in data {
            let value = exec.eval(expr);

            if let Ok(expected) = expected {
                let Ok(value) = value else {
                    panic!("invalid result after eval");
                };
                assert!(matches!(
                    value.try_eq(&expected),
                    Ok(ExprValue::Boolean(true))
                ));
                continue;
            }

            if expected.is_err() && value.is_ok() {
                panic!("invalid result after eval");
            }
        }
    }

    #[test]
    pub fn greater_operator_eval_success() {
        let data: Vec<(&str, Result<ExprValue>)> = vec![
            ("${{ 4 > 4.0 }}", Ok(ExprValue::Boolean(false))),
            ("${{ 4 > 4 }}", Ok(ExprValue::Boolean(false))),
            ("${{ 4 > 5 }}", Ok(ExprValue::Boolean(false))),
            ("${{ 10 > 4.0 }}", Ok(ExprValue::Boolean(true))),
            ("${{ 10 > 9.8 }}", Ok(ExprValue::Boolean(true))),
            ("${{ 10 > 7.2 }}", Ok(ExprValue::Boolean(true))),
            ("${{ true > 5 }}", Err(anyhow!(""))),
            ("${{ 5 > true }}", Err(anyhow!(""))),
            ("${{ \"hello\" > true }}", Err(anyhow!(""))),
            ("${{ false > \"world\" }}", Err(anyhow!(""))),
            ("${{ false > true }}", Ok(ExprValue::Boolean(false))),
            (
                "${{ \"hello\" > \"world\" }}",
                Ok(ExprValue::Boolean(false)),
            ),
        ];

        let mut wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);

        for (expr, expected) in data {
            let value = exec.eval(expr);

            if let Ok(expected) = expected {
                let Ok(value) = value else {
                    panic!("invalid result after eval");
                };
                assert!(matches!(
                    value.try_eq(&expected),
                    Ok(ExprValue::Boolean(true))
                ));
                continue;
            }

            if expected.is_err() && value.is_ok() {
                panic!("invalid result after eval");
            }
        }
    }

    #[test]
    pub fn greater_equals_operator_eval_success() {
        let data: Vec<(&str, Result<ExprValue>)> = vec![
            ("${{ 4 >= 4.0 }}", Ok(ExprValue::Boolean(true))),
            ("${{ 4 >= 4 }}", Ok(ExprValue::Boolean(true))),
            ("${{ 4 >= 5 }}", Ok(ExprValue::Boolean(false))),
            ("${{ 10 >= 4.0 }}", Ok(ExprValue::Boolean(true))),
            ("${{ 10 >= 9.8 }}", Ok(ExprValue::Boolean(true))),
            ("${{ 10 >= 7.2 }}", Ok(ExprValue::Boolean(true))),
            ("${{ true >= 5 }}", Err(anyhow!(""))),
            ("${{ 5 >= true }}", Err(anyhow!(""))),
            ("${{ \"hello\" >= true }}", Err(anyhow!(""))),
            ("${{ false >= \"world\" }}", Err(anyhow!(""))),
            ("${{ false >= true }}", Ok(ExprValue::Boolean(false))),
            ("${{ false >= false }}", Ok(ExprValue::Boolean(true))),
            (
                "${{ \"hello\" >= \"world\" }}",
                Ok(ExprValue::Boolean(false)),
            ),
            (
                "${{ \"hello\" >= \"hello\" }}",
                Ok(ExprValue::Boolean(true)),
            ),
        ];

        let mut wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);

        for (expr, expected) in data {
            let value = exec.eval(expr);

            if let Ok(expected) = expected {
                let Ok(value) = value else {
                    panic!("invalid result after eval");
                };
                assert!(matches!(
                    value.try_eq(&expected),
                    Ok(ExprValue::Boolean(true))
                ));
                continue;
            }

            if expected.is_err() && value.is_ok() {
                panic!("invalid result after eval");
            }
        }
    }

    #[test]
    pub fn less_operator_eval_success() {
        let data: Vec<(&str, Result<ExprValue>)> = vec![
            ("${{ 4 < 4.0 }}", Ok(ExprValue::Boolean(false))),
            ("${{ 4 < 4 }}", Ok(ExprValue::Boolean(false))),
            ("${{ 4 < 5 }}", Ok(ExprValue::Boolean(true))),
            ("${{ 10 < 4.0 }}", Ok(ExprValue::Boolean(false))),
            ("${{ 10 < 9.8 }}", Ok(ExprValue::Boolean(false))),
            ("${{ 10 < 7.2 }}", Ok(ExprValue::Boolean(false))),
            ("${{ true < 5 }}", Err(anyhow!(""))),
            ("${{ 5 < true }}", Err(anyhow!(""))),
            ("${{ \"hello\" < true }}", Err(anyhow!(""))),
            ("${{ false < \"world\" }}", Err(anyhow!(""))),
            ("${{ false < true }}", Ok(ExprValue::Boolean(true))),
            ("${{ \"hello\" < \"world\" }}", Ok(ExprValue::Boolean(true))),
        ];

        let mut wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);

        for (expr, expected) in data {
            let value = exec.eval(expr);

            if let Ok(expected) = expected {
                let Ok(value) = value else {
                    panic!("invalid result after eval");
                };
                assert!(matches!(
                    value.try_eq(&expected),
                    Ok(ExprValue::Boolean(true))
                ));
                continue;
            }

            if expected.is_err() && value.is_ok() {
                panic!("invalid result after eval");
            }
        }
    }

    #[test]
    pub fn less_equals_operator_eval_success() {
        let data: Vec<(&str, Result<ExprValue>)> = vec![
            ("${{ 4 <= 4.0 }}", Ok(ExprValue::Boolean(true))),
            ("${{ 4 <= 4 }}", Ok(ExprValue::Boolean(true))),
            ("${{ 4 <= 5 }}", Ok(ExprValue::Boolean(true))),
            ("${{ 10 <= 4.0 }}", Ok(ExprValue::Boolean(false))),
            ("${{ 10 <= 9.8 }}", Ok(ExprValue::Boolean(false))),
            ("${{ 10 <= 7.2 }}", Ok(ExprValue::Boolean(false))),
            ("${{ true <= 5 }}", Err(anyhow!(""))),
            ("${{ 5 <= true }}", Err(anyhow!(""))),
            ("${{ \"hello\" <= true }}", Err(anyhow!(""))),
            ("${{ false <= \"world\" }}", Err(anyhow!(""))),
            ("${{ false <= true }}", Ok(ExprValue::Boolean(true))),
            ("${{ false <= false }}", Ok(ExprValue::Boolean(true))),
            (
                "${{ \"hello\" <= \"world\" }}",
                Ok(ExprValue::Boolean(true)),
            ),
            (
                "${{ \"hello\" <= \"hello\" }}",
                Ok(ExprValue::Boolean(true)),
            ),
        ];

        let mut wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);

        for (expr, expected) in data {
            let value = exec.eval(expr);

            if let Ok(expected) = expected {
                let Ok(value) = value else {
                    panic!("invalid result after eval");
                };
                dbg!(expr);
                assert!(matches!(
                    value.try_eq(&expected),
                    Ok(ExprValue::Boolean(true))
                ));
                continue;
            }

            if expected.is_err() && value.is_ok() {
                panic!("invalid result after eval");
            }
        }
    }

    #[test]
    pub fn and_logical_expression_eval_success() {
        let data: Vec<(&str, Result<ExprValue>)> = vec![
            (
                "${{ 4 == 4 && true == true }}",
                Ok(ExprValue::Boolean(true)),
            ),
            (
                "${{ 4 == 4 && false == true }}",
                Ok(ExprValue::Boolean(false)),
            ),
            (
                "${{ 5 == 4 && false == true }}",
                Ok(ExprValue::Boolean(false)),
            ),
            (
                "${{ 5 >= 4 && false >= true }}",
                Ok(ExprValue::Boolean(false)),
            ),
            (
                "${{ 5 >= 4 && false <= true }}",
                Ok(ExprValue::Boolean(true)),
            ),
            (
                "${{ \"hello\" >= \"hello\" && false <= true && 42 > 41 }}",
                Ok(ExprValue::Boolean(true)),
            ),
            (
                "${{ \"hello\" >= \"hello\" && true <= false && 42 > 41 }}",
                Ok(ExprValue::Boolean(false)),
            ),
        ];

        let mut wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);

        for (expr, expected) in data {
            let value = exec.eval(expr);

            if let Ok(expected) = expected {
                let Ok(value) = value else {
                    dbg!(&value);
                    panic!("invalid result after eval");
                };
                dbg!(expr);
                dbg!(&value);
                assert!(matches!(
                    value.try_eq(&expected),
                    Ok(ExprValue::Boolean(true))
                ));
                continue;
            }

            if expected.is_err() && value.is_ok() {
                panic!("invalid result after eval");
            }
        }
    }

    #[test]
    pub fn or_logical_expression_eval_success() {
        let data: Vec<(&str, Result<ExprValue>)> = vec![
            (
                "${{ 4 == 4 || true == true }}",
                Ok(ExprValue::Boolean(true)),
            ),
            (
                "${{ 4 == 4 || false == true }}",
                Ok(ExprValue::Boolean(true)),
            ),
            (
                "${{ 5 == 4 || false == true }}",
                Ok(ExprValue::Boolean(false)),
            ),
            (
                "${{ 5 >= 4 || false >= true }}",
                Ok(ExprValue::Boolean(true)),
            ),
            (
                "${{ 5 >= 4 || false <= true }}",
                Ok(ExprValue::Boolean(true)),
            ),
            (
                "${{ 5 == 4 || false == true }}",
                Ok(ExprValue::Boolean(false)),
            ),
            (
                "${{ \"hello\" >= \"hello\" || false <= true || 42 > 41 }}",
                Ok(ExprValue::Boolean(true)),
            ),
            (
                "${{ \"hello\" >= \"hello\" || true <= false || 42 > 41 }}",
                Ok(ExprValue::Boolean(true)),
            ),
            (
                "${{ \"hello\" > \"hello\" || true < false || 42 < 41 }}",
                Ok(ExprValue::Boolean(false)),
            ),
        ];

        let mut wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);

        for (expr, expected) in data {
            let value = exec.eval(expr);

            if let Ok(expected) = expected {
                let Ok(value) = value else {
                    panic!("invalid result after eval");
                };
                dbg!(expr);
                dbg!(&value);
                assert!(matches!(
                    value.try_eq(&expected),
                    Ok(ExprValue::Boolean(true))
                ));
                continue;
            }

            if expected.is_err() && value.is_ok() {
                panic!("invalid result after eval");
            }
        }
    }

    #[test]
    pub fn complex_logical_expression_eval_success() {
        let data: Vec<(&str, Result<ExprValue>)> = vec![
            (
                "${{ true == true && 42 >= 41 || \"hello\" == \"hello\" }}",
                Ok(ExprValue::Boolean(true)),
            ),
            (
                "${{ true == true && 41 >= 42 || \"hello\" == \"hello\" }}",
                Ok(ExprValue::Boolean(true)),
            ),
            (
                "${{ true == true && 41 >= 42 || \"hello2\" == \"hello\" }}",
                Ok(ExprValue::Boolean(false)),
            ),
            (
                "${{ true == true && 41 >= 42 || \"hello2\" == \"hello\" || 5 == 5 }}",
                Ok(ExprValue::Boolean(true)),
            ),
            (
                "${{ true == true && 41 >= 42 || \"hello2\" == \"hello\" || 3 == 5 }}",
                Ok(ExprValue::Boolean(false)),
            ),
            (
                "${{ true == true && 41 >= 42 || \"hello2\" == \"hello\" || 5 == 5 && false > true }}",
                Ok(ExprValue::Boolean(false)),
            ),
            (
                "${{ true == true && 41 >= 42 || \"hello2\" == \"hello\" || 4 == 5 && false > true }}",
                Ok(ExprValue::Boolean(false)),
            ),
            (
                "${{ true == true && 41 >= 42 || \"hello2\" == \"hello\" || 5 == 5 && true > false }}",
                Ok(ExprValue::Boolean(true)),
            ),
            (
                "${{ true == true && 41 >= 42 || \"hello2\" == \"hello\" || 5 == 5 && true > true }}",
                Ok(ExprValue::Boolean(false)),
            ),
        ];

        let mut wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);

        for (expr, expected) in data {
            let value = exec.eval(expr);

            if let Ok(expected) = expected {
                let Ok(value) = value else {
                    panic!("invalid result after eval");
                };
                dbg!(expr);
                dbg!(&value);
                assert!(matches!(
                    value.try_eq(&expected),
                    Ok(ExprValue::Boolean(true))
                ));
                continue;
            }

            if expected.is_err() && value.is_ok() {
                panic!("invalid result after eval");
            }
        }
    }

    #[test]
    pub fn full_expression_eval_success() {
        let data: Vec<(&str, Result<ExprValue>)> = vec![
            (
                "${{ inputs.name == \"john\" && inputs.surname == \"doe\" }}",
                Ok(ExprValue::Boolean(true)),
            ),
            (
                "${{ inputs.name == \"josh\" && inputs.surname == \"doe\" || inputs.age >= \"42\" }}",
                Ok(ExprValue::Boolean(false)),
            ),
            (
                "${{ inputs.name == \"josh\" && inputs.surname == \"doe\" || inputs.age >= \"29\" }}",
                Ok(ExprValue::Boolean(true)),
            ),
        ];

        let mut wctx = MockWritableRuntimeExprContext::new();
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

        let exec = CommonExprExecutor::new(&pipeline, &rctx, &mut wctx);

        for (expr, expected) in data {
            let value = exec.eval(expr);

            if let Ok(expected) = expected {
                let Ok(value) = value else {
                    panic!("invalid result after eval");
                };
                dbg!(expr);
                dbg!(&value);
                assert!(matches!(
                    value.try_eq(&expected),
                    Ok(ExprValue::Boolean(true))
                ));
                continue;
            }

            if expected.is_err() && value.is_ok() {
                panic!("invalid result after eval");
            }
        }
    }
}
