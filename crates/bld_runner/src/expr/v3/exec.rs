use std::collections::HashMap;

use crate::expr::v3::parser::{ExprParser, Rule};

use super::traits::{
    EvalExpr, EvalObject, ExprValue, ReadonlyRuntimeExprContext, WritableRuntimeExprContext,
};
use anyhow::{Result, anyhow, bail};
use pest::{Parser, iterators::Pair};
use regex::Regex;

pub fn eval_all_expressions<'a, E: EvalExpr<'a>>(
    exec: &E,
    regex: &Regex,
    value: &'a str,
) -> Result<String> {
    let mut result = value.to_string();
    for entry in regex.find_iter(value) {
        let entry = entry.as_str();
        let evaluated = exec.eval(entry)?.to_string();
        result = result.replace(entry, &evaluated);
    }
    Ok(result)
}

pub fn eval_all_expressions_map<'a, E: EvalExpr<'a>>(
    exec: &E,
    regex: &Regex,
    values: &'a HashMap<String, String>,
) -> Result<HashMap<String, String>> {
    let mut result = HashMap::new();
    for (name, value) in values {
        result.insert(name.to_owned(), eval_all_expressions(exec, regex, value)?);
    }
    Ok(result)
}

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
    pub fn new(obj_executor: &'a T, rctx: &'a RCtx, wctx: &'a WCtx) -> Self {
        Self {
            obj_executor,
            rctx,
            wctx,
        }
    }

    fn eval_array(&self, expr: Pair<'a, Rule>) -> Result<ExprValue<'a>> {
        let Rule::Array = expr.as_rule() else {
            bail!("expected array rule, found {:?}", expr.as_rule());
        };

        let mut items = Vec::new();
        let mut element_rule: Option<Rule> = None;

        for element in expr.into_inner() {
            let Rule::ArrayElement = element.as_rule() else {
                bail!("expected array element rule, found {:?}", element.as_rule());
            };

            let element = element
                .into_inner()
                .next()
                .ok_or_else(|| anyhow!("empty array element found"))?;
            let rule = element.as_rule();

            match element_rule {
                Some(expected) if expected != rule => {
                    bail!("array elements must all be of the same type")
                }
                None => element_rule = Some(rule),
                _ => {}
            }

            items.push(element.as_span().as_str().try_into()?);
        }

        Ok(ExprValue::Array(items))
    }

    fn eval_and_term(&self, expr: Pair<'a, Rule>) -> Result<ExprValue<'a>> {
        let Rule::AndTerm = expr.as_rule() else {
            bail!("expected and term rule, found {:?}", expr.as_rule());
        };

        let inner = expr
            .into_inner()
            .next()
            .ok_or_else(|| anyhow!("no expression found in and term"))?;

        match inner.as_rule() {
            Rule::AndExpression => self.eval_and_expr(inner),
            Rule::Expression => self.eval_expr(inner),
            _ => bail!("unexpected rule: {:?}", inner.as_rule()),
        }
    }

    fn eval_and_expr(&self, expr: Pair<'a, Rule>) -> Result<ExprValue<'a>> {
        let Rule::AndExpression = expr.as_rule() else {
            bail!("expected and expression rule, found {:?}", expr.as_rule());
        };

        let mut inner = expr.into_inner();

        let first = inner
            .next()
            .ok_or_else(|| anyhow!("no left operand found for and expression"))?;
        let mut result = self.eval_expr(first)?;

        while let Some(operator) = inner.next() {
            let Rule::AndOperator = operator.as_rule() else {
                bail!(
                    "invalid operator encountered during evaluation of and expression: {:?}",
                    operator.as_rule()
                );
            };

            let right = inner
                .next()
                .ok_or_else(|| anyhow!("no right operand found for and expression"))?;

            // short circuit: once the left side is false the overall result is
            // false, so the right side must not be evaluated at all.
            if matches!(result, ExprValue::Boolean(false)) {
                continue;
            }

            let value = self.eval_expr(right)?;
            result = result.try_and(&value)?;
        }

        Ok(result)
    }

    fn eval_index(&self, object: Pair<'a, Rule>, value: ExprValue<'a>) -> Result<ExprValue<'a>> {
        let Some(index) = object
            .into_inner()
            .find(|part| part.as_rule() == Rule::Index)
        else {
            return Ok(value);
        };

        let index_span = index.as_span().as_str();
        let index: usize = index_span[1..index_span.len() - 1]
            .parse()
            .map_err(|_| anyhow!("invalid array index: {index_span}"))?;

        let items = match value {
            ExprValue::Array(items) => items,

            // The real value of a step output or an input is not known during
            // validation, so an index into it can't be checked either.
            ExprValue::Unknown => return Ok(ExprValue::Unknown),

            ExprValue::Text(text) => {
                let raw = text.inner();

                // The validator has no real value for an input, so it stands in with
                // an empty string. An index into that placeholder must not be
                // reported as an error, only the run itself can check it.
                if raw.is_empty() && self.rctx.is_validation() {
                    return Ok(ExprValue::Unknown);
                }

                let parsed: ExprValue<'a> = raw
                    .try_into()
                    .map_err(|_| anyhow!("the value is not an array: '{raw}'"))?;
                let ExprValue::Array(items) = parsed else {
                    bail!("the value is not an array: '{raw}'");
                };
                items
            }

            other => bail!(
                "cannot index into a value of type {}",
                other.type_as_string()
            ),
        };

        let len = items.len();
        items
            .into_iter()
            .nth(index)
            .ok_or_else(|| anyhow!("index {index} out of bounds for array of length {len}"))
    }
}

impl<'a, T: EvalObject<'a>, RCtx: ReadonlyRuntimeExprContext<'a>, WCtx: WritableRuntimeExprContext>
    EvalExpr<'a> for CommonExprExecutor<'a, T, RCtx, WCtx>
{
    fn eval_cmp(&self, expr: Pair<'a, Rule>) -> Result<ExprValue<'a>> {
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

    fn eval_symbol(&self, expr: Pair<'a, Rule>) -> Result<ExprValue<'a>> {
        let Rule::Symbol = expr.as_rule() else {
            bail!("expected symbol rule, found {:?}", expr.as_rule());
        };

        let mut symbol = expr.into_inner().peekable();
        let peeked_symbol = symbol
            .peek()
            .ok_or_else(|| anyhow!("no symbol found in expression"))?;
        let symbol_span = peeked_symbol.as_span();
        let symbol_rule = peeked_symbol.as_rule();
        let object_pair = peeked_symbol.clone();

        match &symbol_rule {
            Rule::Boolean | Rule::Number | Rule::String => symbol_span.as_str().try_into(),
            Rule::Array => {
                let array = symbol
                    .next()
                    .ok_or_else(|| anyhow!("no array found in expression"))?;
                self.eval_array(array)
            }
            Rule::Object => {
                let value = self
                    .obj_executor
                    .eval_object(&mut symbol, self.rctx, self.wctx)?;
                self.eval_index(object_pair, value)
            }
            _ => bail!("unexpected rule: {:?}", &symbol_rule),
        }
    }

    fn eval_expr(&self, expr: Pair<'a, Rule>) -> Result<ExprValue<'a>> {
        let Rule::Expression = expr.as_rule() else {
            bail!("expected expression rule, found {:?}", expr.as_rule());
        };

        let expr_inner = expr
            .into_inner()
            .next()
            .ok_or_else(|| anyhow!("no expression found"))?;

        match expr_inner.as_rule() {
            Rule::LogicalExpression => self.eval_logical_expr(expr_inner),

            Rule::ExpressionInner => {
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

            _ => bail!(
                "expected expression inner or logical expression rule, found {:?}",
                expr_inner.as_rule()
            ),
        }
    }

    fn eval_logical_expr(&self, expr: Pair<'a, Rule>) -> Result<ExprValue<'a>> {
        let Rule::LogicalExpression = expr.as_rule() else {
            bail!(
                "expected logical expression rule, found {:?}",
                expr.as_rule()
            );
        };

        let mut inner = expr.into_inner();

        let first = inner
            .next()
            .ok_or_else(|| anyhow!("no left operand found for logical expression"))?;
        let mut result = self.eval_and_term(first)?;

        while let Some(operator) = inner.next() {
            let Rule::OrOperator = operator.as_rule() else {
                bail!(
                    "invalid operator encountered during evaluation of logical expression: {:?}",
                    operator.as_rule()
                );
            };

            let right = inner
                .next()
                .ok_or_else(|| anyhow!("no right operand found for logical expression"))?;

            // short circuit: once the left side is true the overall result is
            // true, so the right side must not be evaluated at all.
            if matches!(result, ExprValue::Boolean(true)) {
                continue;
            }

            let value = self.eval_and_term(right)?;
            result = result.try_or(&value)?;
        }

        Ok(result)
    }

    fn eval(&self, expr: &'a str) -> Result<ExprValue<'a>> {
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
            traits::{ExprText, MockWritableRuntimeExprContext, OutputScope, tests::expr_number},
        },
        job::v3::Job,
        pipeline::v3::Pipeline,
        runner::v3::{JobState, RootState},
        step::v3::{ShellCommand, Step},
    };
    use anyhow::Result;
    use bld_utils::sync::IntoArc;
    use mockall::predicate;
    use std::collections::HashMap;

    use super::*;

    fn pipeline_with_step(job: &str, step: &str) -> Pipeline {
        let mut pipeline = Pipeline::default();
        pipeline.jobs.insert(
            job.to_string(),
            Job {
                steps: vec![Step::ComplexSh(Box::new(ShellCommand {
                    id: step.to_string(),
                    ..Default::default()
                }))],
                ..Default::default()
            },
        );
        pipeline
    }

    fn rctx_with(
        inputs: Vec<(&str, &str)>,
        env: Vec<(&str, &str)>,
    ) -> CommonReadonlyRuntimeExprContext {
        let owned = |values: Vec<(&str, &str)>| -> HashMap<String, String> {
            values
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        };

        CommonReadonlyRuntimeExprContext {
            inputs: owned(inputs).into_arc(),
            env: owned(env).into_arc(),
            ..Default::default()
        }
    }

    #[test]
    pub fn number_eval_success() {
        let data = vec![
            ("${{ 100 }}", expr_number(100.0)),
            ("${{ 100.0 }}", expr_number(100.0)),
            ("${{ 150.20 }}", expr_number(150.20)),
            ("${{ 0.0 }}", expr_number(0.0)),
            ("${{ 0 }}", expr_number(0.0)),
            ("${{ -100 }}", expr_number(-100.0)),
            ("${{ -100.0 }}", expr_number(-100.0)),
            ("${{ -150.20 }}", expr_number(-150.20)),
        ];

        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        for (expr, expected) in data {
            let Ok(value) = exec.eval(expr) else {
                panic!("failed to parse expression: {expr}");
            };

            let ExprValue::Number { value, .. } = value else {
                panic!("expected number, found {:?}", value);
            };

            let ExprValue::Number {
                value: expected, ..
            } = expected
            else {
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

        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

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
            (
                "${{ \"hello\" }}",
                ExprValue::Text(ExprText::Owned("hello".to_string())),
            ),
            (
                "${{ 'hello' }}",
                ExprValue::Text(ExprText::Owned("hello".to_string())),
            ),
            (
                "${{ \"\" }}",
                ExprValue::Text(ExprText::Owned("".to_string())),
            ),
            (
                "${{ '' }}",
                ExprValue::Text(ExprText::Owned("".to_string())),
            ),
            (
                "${{ \"it's\" }}",
                ExprValue::Text(ExprText::Owned("it's".to_string())),
            ),
            (
                "${{ 'say \"hi\"' }}",
                ExprValue::Text(ExprText::Owned("say \"hi\"".to_string())),
            ),
            (
                "${{ \"say \\\"hi\\\"\" }}",
                ExprValue::Text(ExprText::Owned("say \"hi\"".to_string())),
            ),
            (
                "${{ \"a\\nb\\tc\" }}",
                ExprValue::Text(ExprText::Owned("a\nb\tc".to_string())),
            ),
            (
                "${{ \"back\\\\slash\" }}",
                ExprValue::Text(ExprText::Owned("back\\slash".to_string())),
            ),
        ];

        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

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
    pub fn array_literal_eval_success() {
        let data = vec![
            (
                "${{ [100, 200, 300] }}",
                vec![expr_number(100.0), expr_number(200.0), expr_number(300.0)],
            ),
            (
                "${{ [\"hello\", \"world\"] }}",
                vec![
                    ExprValue::Text(ExprText::Owned("hello".to_string())),
                    ExprValue::Text(ExprText::Owned("world".to_string())),
                ],
            ),
            (
                "${{ [true, false, true] }}",
                vec![
                    ExprValue::Boolean(true),
                    ExprValue::Boolean(false),
                    ExprValue::Boolean(true),
                ],
            ),
        ];

        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        for (expr, expected) in data {
            let Ok(value) = exec.eval(expr) else {
                panic!("failed to parse expression: {expr}");
            };

            let ExprValue::Array(items) = value else {
                panic!("expected array, found {:?}", value);
            };

            assert_eq!(items.len(), expected.len());
            for (item, expected_item) in items.iter().zip(expected.iter()) {
                assert!(matches!(
                    item.try_eq(expected_item),
                    Ok(ExprValue::Boolean(true))
                ));
            }
        }
    }

    #[test]
    pub fn array_mixed_type_eval_failure() {
        let data = [
            "${{ [1, \"two\", true] }}",
            "${{ [1, 2, \"three\"] }}",
            "${{ [true, 2] }}",
        ];

        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        for expr in data {
            assert!(
                exec.eval(expr).is_err(),
                "expected error for mixed type array: {expr}"
            );
        }
    }

    #[test]
    pub fn array_equals_eval_success() {
        let data: Vec<(&str, Result<ExprValue>)> = vec![
            (
                "${{ [1, 2, 3] == [1, 2, 3] }}",
                Ok(ExprValue::Boolean(true)),
            ),
            (
                "${{ [1, 2, 3] == [1, 2, 4] }}",
                Ok(ExprValue::Boolean(false)),
            ),
            ("${{ [1, 2, 3] == [1, 2] }}", Ok(ExprValue::Boolean(false))),
            (
                "${{ [\"a\", \"b\"] == [\"a\", \"b\"] }}",
                Ok(ExprValue::Boolean(true)),
            ),
            (
                "${{ [true, false] == [true, false] }}",
                Ok(ExprValue::Boolean(true)),
            ),
            ("${{ [1, 2, 3] == 5 }}", Err(anyhow!(""))),
        ];

        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        for (expr, expected) in data {
            let value = exec.eval(expr);

            if let Ok(expected) = expected {
                let Ok(value) = value else {
                    panic!("invalid result after eval for {expr}");
                };
                assert!(matches!(
                    value.try_eq(&expected),
                    Ok(ExprValue::Boolean(true))
                ));
                continue;
            }

            if expected.is_err() && value.is_ok() {
                panic!("expected an error for {expr}");
            }
        }
    }

    #[test]
    pub fn array_not_equals_eval_success() {
        let data: Vec<(&str, Result<ExprValue>)> = vec![
            (
                "${{ [1, 2, 3] != [1, 2, 3] }}",
                Ok(ExprValue::Boolean(false)),
            ),
            (
                "${{ [1, 2, 3] != [1, 2, 4] }}",
                Ok(ExprValue::Boolean(true)),
            ),
            ("${{ [1, 2, 3] != [1, 2] }}", Ok(ExprValue::Boolean(true))),
        ];

        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        for (expr, expected) in data {
            let value = exec.eval(expr);

            if let Ok(expected) = expected {
                let Ok(value) = value else {
                    panic!("invalid result after eval for {expr}");
                };
                assert!(matches!(
                    value.try_eq(&expected),
                    Ok(ExprValue::Boolean(true))
                ));
                continue;
            }

            if expected.is_err() && value.is_ok() {
                panic!("expected an error for {expr}");
            }
        }
    }

    #[test]
    pub fn array_other_comparisons_eval_failure() {
        let data = [
            "${{ [1, 2, 3] > [1, 2, 3] }}",
            "${{ [1, 2, 3] >= [1, 2, 3] }}",
            "${{ [1, 2, 3] < [1, 2, 3] }}",
            "${{ [1, 2, 3] <= [1, 2, 3] }}",
        ];

        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        for expr in data {
            assert!(
                exec.eval(expr).is_err(),
                "expected error for comparison operator on arrays: {expr}"
            );
        }
    }

    #[test]
    pub fn array_index_access_eval_success() {
        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = rctx_with(
            vec![
                ("names", "[\"john\", \"jane\", \"jim\"]"),
                ("numbers", "[100, 200, 300]"),
                ("flags", "[true, false]"),
            ],
            vec![],
        );

        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        let Ok(value) = exec.eval("${{ inputs.names[1] }}") else {
            panic!("failed to eval indexed expression");
        };
        assert!(matches!(
            value.try_eq(&ExprValue::Text(ExprText::Owned("jane".to_string()))),
            Ok(ExprValue::Boolean(true))
        ));

        let Ok(value) = exec.eval("${{ inputs.numbers[2] }}") else {
            panic!("failed to eval indexed expression");
        };
        assert!(matches!(
            value.try_eq(&expr_number(300.0)),
            Ok(ExprValue::Boolean(true))
        ));

        let Ok(value) = exec.eval("${{ inputs.flags[0] }}") else {
            panic!("failed to eval indexed expression");
        };
        assert!(matches!(
            value.try_eq(&ExprValue::Boolean(true)),
            Ok(ExprValue::Boolean(true))
        ));
    }

    #[test]
    pub fn array_index_access_out_of_bounds_eval_failure() {
        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = rctx_with(vec![("numbers", "[100, 200, 300]")], vec![]);

        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        let err = exec.eval("${{ inputs.numbers[5] }}").unwrap_err();
        assert!(err.to_string().contains("length 3"), "error was: {err}");
    }

    #[test]
    pub fn array_step_output_index_access_eval_success() {
        let mut wctx = MockWritableRuntimeExprContext::new();
        wctx.expect_get_exec_id().returning(|| Some("main"));
        wctx.expect_get_output()
            .with(
                predicate::eq(OutputScope::Step),
                predicate::eq("build"),
                predicate::eq("items"),
            )
            .times(1)
            .returning(|_, _, _| {
                Ok(ExprValue::Array(vec![
                    ExprValue::Text(ExprText::Ref("x")),
                    ExprValue::Text(ExprText::Ref("y")),
                ]))
            });

        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = pipeline_with_step("main", "build");
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        let value = exec
            .eval("${{ steps.build.outputs.items[0] }}")
            .expect("failed to eval indexed step output");
        assert!(matches!(
            value.try_eq(&ExprValue::Text(ExprText::Owned("x".to_string()))),
            Ok(ExprValue::Boolean(true))
        ));
    }

    /// The state of a run converts the text of an output into a value, so an array
    /// output has to be indexable exactly as it is stored there.
    #[test]
    pub fn array_step_output_of_run_state_index_access_eval_success() {
        let mut wctx = JobState::new("main");
        wctx.add_node("build");
        wctx.set_output("build", "items".to_string(), "[\"x\", \"y\"]".to_string())
            .unwrap();

        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = pipeline_with_step("main", "build");
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        let value = exec
            .eval("${{ steps.build.outputs.items[1] }}")
            .expect("failed to eval indexed step output");
        assert_eq!(value.to_string(), "y");
    }

    #[test]
    pub fn text_step_output_index_access_eval_success() {
        let mut wctx = MockWritableRuntimeExprContext::new();
        wctx.expect_get_exec_id().returning(|| Some("main"));
        wctx.expect_get_output()
            .with(
                predicate::eq(OutputScope::Step),
                predicate::eq("build"),
                predicate::eq("items"),
            )
            .times(1)
            .returning(|_, _, _| {
                Ok(ExprValue::Text(ExprText::Owned(
                    "[\"x\", \"y\"]".to_string(),
                )))
            });

        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = pipeline_with_step("main", "build");
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        let value = exec
            .eval("${{ steps.build.outputs.items[1] }}")
            .expect("failed to eval indexed step output");
        assert!(matches!(
            value.try_eq(&ExprValue::Text(ExprText::Owned("y".to_string()))),
            Ok(ExprValue::Boolean(true))
        ));
    }

    #[test]
    pub fn index_into_number_step_output_eval_failure() {
        let mut wctx = MockWritableRuntimeExprContext::new();
        wctx.expect_get_exec_id().returning(|| Some("main"));
        wctx.expect_get_output()
            .with(
                predicate::eq(OutputScope::Step),
                predicate::eq("build"),
                predicate::eq("count"),
            )
            .times(1)
            .returning(|_, _, _| {
                Ok(ExprValue::Number {
                    value: 5.0,
                    raw: ExprText::Owned("5".to_string()),
                })
            });

        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = pipeline_with_step("main", "build");
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        let err = exec
            .eval("${{ steps.build.outputs.count[0] }}")
            .unwrap_err();
        assert!(err.to_string().contains("number"), "error was: {err}");
    }

    #[test]
    pub fn index_into_blank_validation_placeholder_gives_unknown() {
        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = rctx_with(vec![("list", "")], vec![]).with_validation();

        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        let value = exec
            .eval("${{ inputs.list[0] }}")
            .expect("indexing a blank placeholder value must not error");
        assert!(matches!(value, ExprValue::Unknown));
    }

    /// Outside of validation a blank value is a real one, so an index into it has
    /// to be reported instead of quietly giving an unknown value.
    #[test]
    pub fn index_into_blank_input_of_a_run_eval_failure() {
        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = rctx_with(vec![("list", "")], vec![]);

        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        let err = exec.eval("${{ inputs.list[0] }}").unwrap_err();
        assert!(
            err.to_string().contains("the value is not an array: ''"),
            "error was: {err}"
        );
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

        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = rctx_with(
            vec![("name", "John"), ("surname", "Doe"), ("age", "32")],
            vec![("WORKDIR", "/home/somedir"), ("NODE", "lts")],
        );

        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

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

        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

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

        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

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

        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

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

        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

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

        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

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

        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

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

        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

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

        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

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

        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

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
    pub fn and_has_higher_precedence_than_or_eval_success() {
        let data: Vec<(&str, Result<ExprValue>)> = vec![
            // true || (false && false) == true, not (true || false) && false == false
            (
                "${{ true || false && false }}",
                Ok(ExprValue::Boolean(true)),
            ),
            // (false && true) || true == true
            ("${{ false && true || true }}", Ok(ExprValue::Boolean(true))),
            // explicit parens force the opposite grouping and change the result
            (
                "${{ (true || false) && false }}",
                Ok(ExprValue::Boolean(false)),
            ),
            // true || (true && false) == true, a left fold would give false
            ("${{ true || true && false }}", Ok(ExprValue::Boolean(true))),
            // false || (true && true) || (false && false) == true, a left fold
            // would give false
            (
                "${{ false || true && true || false && false }}",
                Ok(ExprValue::Boolean(true)),
            ),
            // the same precedence applies to comparisons used as operands
            (
                "${{ 1 == 1 || 2 == 2 && 3 == 4 }}",
                Ok(ExprValue::Boolean(true)),
            ),
        ];

        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        for (expr, expected) in data {
            let value = exec.eval(expr);

            if let Ok(expected) = expected {
                let Ok(value) = value else {
                    panic!("invalid result after eval for expr: {expr}");
                };
                assert!(
                    matches!(value.try_eq(&expected), Ok(ExprValue::Boolean(true))),
                    "unexpected result for expr: {expr}"
                );
                continue;
            }

            if expected.is_err() && value.is_ok() {
                panic!("invalid result after eval for expr: {expr}");
            }
        }
    }

    #[test]
    pub fn logical_operators_short_circuit_eval_success() {
        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        // the right side of `&&` references a missing step output, which would
        // normally error. since the left side is false, the right side must
        // never be evaluated, so no error should be raised.
        let value = exec
            .eval("${{ false && steps.x.outputs.missing == \"1\" }}")
            .expect("expected the short circuited && expression to evaluate without error");
        assert!(matches!(
            value.try_eq(&ExprValue::Boolean(false)),
            Ok(ExprValue::Boolean(true))
        ));

        // the right side of `||` references a missing step output, which would
        // normally error. since the left side is true, the right side must
        // never be evaluated, so no error should be raised.
        let value = exec
            .eval("${{ true || steps.x.outputs.missing == \"1\" }}")
            .expect("expected the short circuited || expression to evaluate without error");
        assert!(matches!(
            value.try_eq(&ExprValue::Boolean(true)),
            Ok(ExprValue::Boolean(true))
        ));
    }

    #[test]
    pub fn non_boolean_logical_operand_eval_failure() {
        let data = [
            "${{ true && 100 }}",
            "${{ 100 && true }}",
            "${{ false || \"hello\" }}",
            "${{ \"hello\" || true }}",
            "${{ true && true && 100 }}",
        ];

        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        for expr in data {
            assert!(
                exec.eval(expr).is_err(),
                "expected an error for expr: {expr}"
            );
        }
    }

    #[test]
    pub fn nested_parens_logical_expression_eval_success() {
        let data: Vec<(&str, Result<ExprValue>)> = vec![
            (
                "${{ (true == true && false == false) }}",
                Ok(ExprValue::Boolean(true)),
            ),
            (
                "${{ (true == true && false == true) }}",
                Ok(ExprValue::Boolean(false)),
            ),
            (
                "${{ true || (false && false) }}",
                Ok(ExprValue::Boolean(true)),
            ),
            (
                "${{ false || (false && false) }}",
                Ok(ExprValue::Boolean(false)),
            ),
            (
                "${{ (true && false) || true }}",
                Ok(ExprValue::Boolean(true)),
            ),
            (
                "${{ true && (false || true) }}",
                Ok(ExprValue::Boolean(true)),
            ),
            (
                "${{ (true == true) && (false == false) }}",
                Ok(ExprValue::Boolean(true)),
            ),
            (
                "${{ (5 > 4 && 3 > 2) || (1 > 2 && 2 > 1) }}",
                Ok(ExprValue::Boolean(true)),
            ),
            (
                "${{ (5 > 4 && 3 < 2) || (1 > 2 && 2 > 1) }}",
                Ok(ExprValue::Boolean(false)),
            ),
        ];

        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

        for (expr, expected) in data {
            let value = exec.eval(expr);

            if let Ok(expected) = expected {
                let Ok(value) = value else {
                    panic!("invalid result after eval for expr: {expr}");
                };
                assert!(
                    matches!(value.try_eq(&expected), Ok(ExprValue::Boolean(true))),
                    "unexpected result for expr: {expr}"
                );
                continue;
            }

            if expected.is_err() && value.is_ok() {
                panic!("invalid result after eval for expr: {expr}");
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

        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = rctx_with(
            vec![("name", "john"), ("surname", "doe"), ("age", "30")],
            vec![],
        );

        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

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
}
