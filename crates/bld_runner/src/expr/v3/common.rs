use crate::expr::v3::parser::{ExprParser, Rule};

use super::traits::{EvalExpr, EvalObject, ExprValue, RuntimeExecutionContext};
use anyhow::{Result, anyhow, bail};
use pest::{Parser, iterators::Pair};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct CommonRuntimeExecutionContext<'a> {
    root_dir: &'a str,
    project_dir: &'a str,
    inputs: HashMap<&'a str, &'a str>,
    outputs: HashMap<&'a str, &'a str>,
    env: HashMap<&'a str, &'a str>,
    run_id: &'a str,
    run_start_time: &'a str,
}

impl<'a> RuntimeExecutionContext<'a> for CommonRuntimeExecutionContext<'a> {
    fn get_root_dir(&self) -> &'a str {
        self.root_dir
    }

    fn get_project_dir(&self) -> &'a str {
        self.project_dir
    }

    fn get_input(&'a self, name: &'a str) -> Result<&'a str> {
        self.inputs
            .get(name)
            .copied()
            .ok_or_else(|| anyhow!("input '{name}' not found"))
    }

    fn get_output(&self, name: &'a str) -> Result<&'a str> {
        self.outputs
            .get(name)
            .copied()
            .ok_or_else(|| anyhow!("output '{name}' not found"))
    }

    fn set_output(&mut self, name: &'a str, value: &'a str) -> Result<()> {
        self.outputs.insert(name, value);
        Ok(())
    }

    fn get_env(&'a self, name: &'a str) -> Result<&'a str> {
        self.env
            .get(name)
            .copied()
            .ok_or_else(|| anyhow!("env variable '{name}' not found"))
    }

    fn get_run_id(&self) -> &'a str {
        self.run_id
    }

    fn get_run_start_time(&self) -> &'a str {
        self.run_start_time
    }
}

pub struct CommonExprExecutor<'a, T: EvalObject<'a>> {
    obj_executor: T,
    ctx: CommonRuntimeExecutionContext<'a>,
}

impl<'a, T: EvalObject<'a>> CommonExprExecutor<'a, T> {
    pub fn new(obj_executor: T, ctx: CommonRuntimeExecutionContext<'a>) -> Self {
        Self {
            obj_executor,
            ctx,
        }
    }
}

impl<'a, T: EvalObject<'a>> EvalExpr<'a> for CommonExprExecutor<'a, T> {
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
            Rule::Equals => left.try_eq(&right),

            Rule::Greater => left.try_ord(&right),

            Rule::GreaterEquals => left.try_ord(&right).and_then(|v| {
                if matches!(v, ExprValue::Boolean(false)) {
                    left.try_eq(&right)
                } else {
                    Ok(v)
                }
            }),

            Rule::Less => right.try_ord(&left),

            Rule::LessEquals => right.try_ord(&left).and_then(|v| {
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
            Rule::Object => self.obj_executor.eval_object(&mut symbol, &self.ctx),
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

    fn eval_logical_expr(&'a self, _expr: Pair<'_, Rule>) -> Result<ExprValue<'a>> {
        unimplemented!()
    }

    fn eval(&'a mut self, expr: &'a str) -> Result<ExprValue<'a>> {
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
