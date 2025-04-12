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
