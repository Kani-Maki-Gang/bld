use std::collections::HashMap;

use crate::expr::v3::parser::{ExprParser, Rule};

use super::traits::{
    EvalExpr, EvalObject, ExprText, ExprValue, ReadonlyRuntimeExprContext,
    WritableRuntimeExprContext,
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

/// Resolves a map of name to expression against the current runtime scope. Used both to
/// resolve the `with`/`env` values sent into a child pipeline or action, and to resolve the
/// `outputs` of an action once all of its steps have completed.
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

        let type_str = value.type_as_string();
        let ExprValue::Text(ExprText::Ref(text)) = value else {
            bail!("cannot index into value of type {type_str}");
        };

        let mut pairs = ExprParser::parse(Rule::Array, text)
            .map_err(|_| anyhow!("value is not an array: {text}"))?;
        let array = pairs
            .next()
            .ok_or_else(|| anyhow!("value is not an array: {text}"))?;

        let ExprValue::Array(items) = self.eval_array(array)? else {
            bail!("expected array value");
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
            traits::{ExprText, MockWritableRuntimeExprContext},
        },
        pipeline::v3::Pipeline,
    };
    use anyhow::Result;
    use bld_utils::sync::IntoArc;
    use std::collections::HashMap;

    use super::*;

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
            ("${{ 100 }}", ExprValue::Number(100.0)),
            ("${{ 100.0 }}", ExprValue::Number(100.0)),
            ("${{ 150.20 }}", ExprValue::Number(150.20)),
            ("${{ 0.0 }}", ExprValue::Number(0.0)),
            ("${{ 0 }}", ExprValue::Number(0.0)),
            ("${{ -100 }}", ExprValue::Number(-100.0)),
            ("${{ -100.0 }}", ExprValue::Number(-100.0)),
            ("${{ -150.20 }}", ExprValue::Number(-150.20)),
        ];

        let wctx = MockWritableRuntimeExprContext::new();
        let rctx = CommonReadonlyRuntimeExprContext::default();
        let pipeline = Pipeline::default();
        let exec = CommonExprExecutor::new(&pipeline, &rctx, &wctx);

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
                vec![
                    ExprValue::Number(100.0),
                    ExprValue::Number(200.0),
                    ExprValue::Number(300.0),
                ],
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
            value.try_eq(&ExprValue::Number(300.0)),
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

        assert!(exec.eval("${{ inputs.numbers[5] }}").is_err());
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
