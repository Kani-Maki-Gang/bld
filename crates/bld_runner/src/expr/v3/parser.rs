#![allow(dead_code)]

use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar/expr_v3.pest"]
pub struct ExprParser;

#[cfg(test)]
mod tests {
    use super::{ExprParser, Rule};
    use pest::Parser;

    #[test]
    fn parse_equals_operator_success() {
        let data = ["==", " ==", "== ", " == "];
        for op in data {
            let Ok(pairs) = ExprParser::parse(Rule::EqualsOperator, op) else {
                panic!("unable to parse EqualsOperator");
            };
            for pair in pairs {
                let Rule::EqualsOperator = pair.as_rule() else {
                    panic!("parsed value is not a EqualsOperator rule");
                };
                assert_eq!(pair.as_span().as_str(), op);
            }
        }
    }

    #[test]
    fn parse_greater_operator_success() {
        let data = [">", " >", "> ", " > "];
        for op in data {
            let Ok(pairs) = ExprParser::parse(Rule::GreaterOperator, op) else {
                panic!("unable to parse GreaterOperator");
            };
            for pair in pairs {
                let Rule::GreaterOperator = pair.as_rule() else {
                    panic!("parsed value is not a GreaterOperator rule");
                };
                assert_eq!(pair.as_span().as_str(), op);
            }
        }
    }

    #[test]
    fn parse_greater_equals_operator_success() {
        let data = [">=", " >=", ">= ", " >= "];
        for op in data {
            let Ok(pairs) = ExprParser::parse(Rule::GreaterEqualsOperator, op) else {
                panic!("unable to parse GreaterEqualsOperator");
            };
            for pair in pairs {
                let Rule::GreaterEqualsOperator = pair.as_rule() else {
                    panic!("parsed value is not a GreaterEqualsOperator rule");
                };
                assert_eq!(pair.as_span().as_str(), op);
            }
        }
    }

    #[test]
    fn parse_less_operator_success() {
        let data = ["<", " <", "< ", " < "];
        for op in data {
            let Ok(pairs) = ExprParser::parse(Rule::LessOperator, op) else {
                panic!("unable to parse LessOperator");
            };
            for pair in pairs {
                let Rule::LessOperator = pair.as_rule() else {
                    panic!("parsed value is not a LessOperator rule");
                };
                assert_eq!(pair.as_span().as_str(), op);
            }
        }
    }

    #[test]
    fn parse_less_equals_operator_success() {
        let data = ["<=", " <=", "<= ", " <= "];
        for op in data {
            let Ok(pairs) = ExprParser::parse(Rule::LessEqualsOperator, op) else {
                panic!("unable to parse LessEqualsOperator");
            };
            for pair in pairs {
                let Rule::LessEqualsOperator = pair.as_rule() else {
                    panic!("parsed value is not a LessEqualsOperator rule");
                };
                assert_eq!(pair.as_span().as_str(), op);
            }
        }
    }

    #[test]
    fn parse_and_operator_success() {
        let data = ["&&", " &&", "&& ", " && "];
        for op in data {
            let Ok(pairs) = ExprParser::parse(Rule::AndOperator, op) else {
                panic!("unable to parse AndOperator");
            };
            for pair in pairs {
                let Rule::AndOperator = pair.as_rule() else {
                    panic!("parsed value is not an AndOperator rule");
                };
                assert_eq!(pair.as_span().as_str(), op);
            }
        }
    }

    #[test]
    fn parse_or_operator_success() {
        let data = ["||", " ||", "|| ", " || "];
        for op in data {
            let Ok(pairs) = ExprParser::parse(Rule::OrOperator, op) else {
                panic!("unable to parse OrOperator");
            };
            for pair in pairs {
                let Rule::OrOperator = pair.as_rule() else {
                    panic!("parsed value is not an OrOperatorrule");
                };
                assert_eq!(pair.as_span().as_str(), op);
            }
        }
    }

    #[test]
    fn parse_number_success() {
        let data = [
            "98",
            "-98",
            "0",
            "156",
            "-156",
            "0.945",
            "-0.945",
            "140.2341",
            "-140.2341",
        ];
        for number in data {
            let Ok(pairs) = ExprParser::parse(Rule::Number, number) else {
                panic!("unable to parse Number symbol");
            };
            for pair in pairs {
                let Rule::Number = pair.as_rule() else {
                    panic!("parsed value is not a Number rule");
                };
                assert_eq!(pair.as_span().get_input(), number);
            }
        }
    }

    #[test]
    fn parse_bool_success() {
        let data = ["true", "false"];
        for boolean in data {
            let Ok(pairs) = ExprParser::parse(Rule::Boolean, boolean) else {
                panic!("unable to parse Boolean symbol");
            };
            for pair in pairs {
                let Rule::Boolean = pair.as_rule() else {
                    panic!("parsed value is not a Boolean rule");
                };
                assert_eq!(pair.as_span().as_str(), boolean);
            }
        }
    }

    #[test]
    fn parse_string_success() {
        let data = ["\"hellow world\""];
        for string in data {
            let Ok(pairs) = ExprParser::parse(Rule::String, string) else {
                panic!("unable to parse String symbol");
            };
            for pair in pairs {
                let Rule::String = pair.as_rule() else {
                    panic!("parsed value is not String rule");
                };
                assert_eq!(pair.as_span().as_str(), string);
            }
        }
    }

    #[test]
    fn parse_object_success() {
        let data = [
            vec!["customer", "customer"],
            vec!["customer.name", "customer", "name"],
            vec!["customer.123", "customer", "123"],
            vec!["customer.name.toString()", "customer", "name", "toString()"],
            vec![
                "customer().name().length()",
                "customer()",
                "name()",
                "length()",
            ],
            vec![
                "customer().name23_23().length()",
                "customer()",
                "name23_23()",
                "length()",
            ],
        ];
        for entry in data.iter() {
            let expr = entry.first().unwrap();

            let Ok(pairs) = ExprParser::parse(Rule::Object, expr) else {
                panic!("unable to parse OBJECT symbol");
            };

            let spans = &entry[1..];

            for object in pairs {
                assert_eq!(object.as_rule(), Rule::Object);
                let object_parts = object.into_inner();
                for (i, object_part) in object_parts.into_iter().enumerate() {
                    assert_eq!(object_part.as_rule(), Rule::ObjectPart);
                    assert_eq!(object_part.as_span().as_str(), spans[i]);
                    assert_eq!(object_part.into_inner().count(), 0 as usize);
                }
            }
        }
    }

    #[test]
    fn parse_symbol_success() {
        let data = [
            vec!["98"],
            vec!["-98"],
            vec!["0"],
            vec!["156"],
            vec!["-156"],
            vec!["0.945"],
            vec!["-0.945"],
            vec!["140.2341"],
            vec!["-140.2341"],
            vec!["true"],
            vec!["false"],
            vec!["\"hello world\""],
            vec!["customer", "customer"],
            vec!["customer.name", "customer", "name"],
            vec!["customer.123", "customer", "123"],
            vec!["customer.name.toString()", "customer", "name", "toString()"],
            vec![
                "customer().name().length()",
                "customer()",
                "name()",
                "length()",
            ],
            vec![
                "customer().name23_23().length()",
                "customer()",
                "name23_23()",
                "length()",
            ],
        ];
        for symbol in data {
            let Ok(pairs) = ExprParser::parse(Rule::Symbol, symbol[0]) else {
                panic!("unable to parse Symbol");
            };
            for pair in pairs {
                let Rule::Symbol = pair.as_rule() else {
                    panic!("parsed value is not a Symbol rule");
                };

                for pair_inner in pair.into_inner() {
                    match pair_inner.as_rule() {
                        Rule::Number | Rule::Boolean | Rule::String => {
                            assert_eq!(pair_inner.as_span().as_str(), symbol[0]);
                        }

                        Rule::Object => {
                            let object_parts = pair_inner.into_inner();
                            for (i, object_part) in object_parts.into_iter().enumerate() {
                                assert_eq!(object_part.as_rule(), Rule::ObjectPart);
                                assert_eq!(object_part.as_span().as_str(), symbol[i + 1]);
                                assert_eq!(object_part.into_inner().count(), 0 as usize);
                            }
                        }

                        _ => panic!("inner pair is not of a valid rule"),
                    }
                }
            }
        }
    }

    #[test]
    fn parse_equals_success() {
        let data = [
            "100 == 150.12",
            "100 == -150.12",
            "-100 == -150.12",
            "-100 == 150.12",
            "100== 150.12",
            "100 ==-150.12",
            "-100==-150.12",
            "100 == true",
            "true== true",
            "true ==false",
            "false==false",
            "\"hello\" == 100",
            "\"hello\" == -100",
            "\"hello\" == true",
            "\"hello\" == false",
            "\"hello\" == customer.name.length",
            "\"hello\" == customer().name().length()",
            "\"hello\" == customer().name23_23().length()",
            "\"hello\" == \"hello world\"",
            "100 == \"hello\"",
            "-100 == \"hello\"",
            "true == \"hello\"",
            "false == \"hello\"",
            "customer.name.length == \"hello\"",
            "\"hello world\" == \"hello\"",
        ];
        for op in data {
            let pair = ExprParser::parse(Rule::Equals, op);
            assert!(pair.is_ok());
        }
    }

    #[test]
    fn parse_greater_success() {
        let data = [
            "100 > 150.12",
            "100 > -150.12",
            "-100 > -150.12",
            "-100 > 150.12",
            "100> 150.12",
            "100 >-150.12",
            "-100>-150.12",
            "100 > true",
            "true> true",
            "true >false",
            "false>false",
            "\"hello\" > 100",
            "\"hello\" > -100",
            "\"hello\" > true",
            "\"hello\" > false",
            "\"hello\" > customer.name.length",
            "\"hello\" > customer.name().length()",
            "\"hello\" > customer().name().length()",
            "\"hello\" > customer().name23_23().length()",
            "\"hello\" > \"hello world\"",
            "100 > \"hello\"",
            "-100 > \"hello\"",
            "true > \"hello\"",
            "false > \"hello\"",
            "customer.name.length > \"hello\"",
            "\"hello world\" > \"hello\"",
        ];
        for op in data {
            let Ok(pairs) = ExprParser::parse(Rule::Greater, op) else {
                panic!("unable to parse Greater rule");
            };

            for pair in pairs {
                let mut pair_inner = pair.into_inner();

                let Some(left) = pair_inner.next() else {
                    panic!("parsed value doesn't contain a left operand");
                };
                assert_eq!(left.as_rule(), Rule::Symbol);

                let Some(operator) = pair_inner.next() else {
                    panic!("parsed value doesn't contain an operator");
                };
                assert_eq!(operator.as_rule(), Rule::GreaterOperator);

                let Some(right) = pair_inner.next() else {
                    panic!("parsed value doesn't contain a right operand");
                };
                assert_eq!(right.as_rule(), Rule::Symbol);
            }
        }
    }

    #[test]
    fn parse_greater_equals_success() {
        let data = [
            "100 >= 150.12",
            "100 >= -150.12",
            "-100 >= -150.12",
            "-100 >= 150.12",
            "100>= 150.12",
            "100 >=-150.12",
            "-100>=-150.12",
            "100 >= true",
            "true>= true",
            "true >=false",
            "false>=false",
            "\"hello\" >= 100",
            "\"hello\" >= -100",
            "\"hello\" >= true",
            "\"hello\" >= false",
            "\"hello\" >= customer.name.length",
            "\"hello\" >= customer.name().length()",
            "\"hello\" >= customer().name().length()",
            "\"hello\" >= customer().name23_23().length()",
            "\"hello\" >= \"hello world\"",
            "100 >= \"hello\"",
            "-100 >= \"hello\"",
            "true >= \"hello\"",
            "false >= \"hello\"",
            "customer.name.length >= \"hello\"",
            "\"hello world\" >= \"hello\"",
        ];
        for op in data {
            let Ok(pairs) = ExprParser::parse(Rule::GreaterEquals, op) else {
                panic!("unable to parse GreaterEquals rule");
            };

            for pair in pairs {
                let mut pair_inner = pair.into_inner();

                let Some(left) = pair_inner.next() else {
                    panic!("parsed value doesn't contain a left operand");
                };
                assert_eq!(left.as_rule(), Rule::Symbol);

                let Some(operator) = pair_inner.next() else {
                    panic!("parsed value doesn't contain an operator");
                };
                assert_eq!(operator.as_rule(), Rule::GreaterEqualsOperator);

                let Some(right) = pair_inner.next() else {
                    panic!("parsed value doesn't contain a right operand");
                };
                assert_eq!(right.as_rule(), Rule::Symbol);
            }
        }
    }

    #[test]
    fn parse_less_success() {
        let data = [
            "100 < 150.12",
            "100 < -150.12",
            "-100 < -150.12",
            "-100 < 150.12",
            "100< 150.12",
            "100 <-150.12",
            "-100<-150.12",
            "100 < true",
            "true< true",
            "true <false",
            "false<false",
            "\"hello\" < 100",
            "\"hello\" < -100",
            "\"hello\" < true",
            "\"hello\" < false",
            "\"hello\" < customer.name.length",
            "\"hello\" < customer.name().length()",
            "\"hello\" < customer().name().length()",
            "\"hello\" < customer().name23_23().length()",
            "\"hello\" < \"hello world\"",
            "100 < \"hello\"",
            "-100 < \"hello\"",
            "true < \"hello\"",
            "false < \"hello\"",
            "customer.name.length < \"hello\"",
            "\"hello world\" < \"hello\"",
        ];
        for op in data {
            let Ok(pairs) = ExprParser::parse(Rule::Less, op) else {
                panic!("unable to parse Less rule");
            };

            for pair in pairs {
                let mut pair_inner = pair.into_inner();

                let Some(left) = pair_inner.next() else {
                    panic!("parsed value doesn't contain a left operand");
                };
                assert_eq!(left.as_rule(), Rule::Symbol);

                let Some(operator) = pair_inner.next() else {
                    panic!("parsed value doesn't contain an operator");
                };
                assert_eq!(operator.as_rule(), Rule::LessOperator);

                let Some(right) = pair_inner.next() else {
                    panic!("parsed value doesn't contain a right operand");
                };
                assert_eq!(right.as_rule(), Rule::Symbol);
            }
        }
    }

    #[test]
    fn parse_less_equals_success() {
        let data = [
            "100 <= 150.12",
            "100 <= -150.12",
            "-100 <= -150.12",
            "-100 <= 150.12",
            "100<= 150.12",
            "100 <=-150.12",
            "-100<=-150.12",
            "100 <= true",
            "true<= true",
            "true <=false",
            "false<=false",
            "\"hello\" <= 100",
            "\"hello\" <= -100",
            "\"hello\" <= true",
            "\"hello\" <= false",
            "\"hello\" <= customer.name.length",
            "\"hello\" <= customer.name().length()",
            "\"hello\" <= customer().name().length()",
            "\"hello\" <= customer().name23_23().length()",
            "\"hello\" <= \"hello world\"",
            "100 <= \"hello\"",
            "-100 <= \"hello\"",
            "true <= \"hello\"",
            "false <= \"hello\"",
            "customer.name.length <= \"hello\"",
            "\"hello world\" <= \"hello\"",
        ];
        for op in data {
            let Ok(pairs) = ExprParser::parse(Rule::LessEquals, op) else {
                panic!("unable to parse LessEquals rule");
            };

            for pair in pairs {
                let mut pair_inner = pair.into_inner();

                let Some(left) = pair_inner.next() else {
                    panic!("parsed value doesn't contain a left operand");
                };
                assert_eq!(left.as_rule(), Rule::Symbol);

                let Some(operator) = pair_inner.next() else {
                    panic!("parsed value doesn't contain an operator");
                };
                assert_eq!(operator.as_rule(), Rule::LessEqualsOperator);

                let Some(right) = pair_inner.next() else {
                    panic!("parsed value doesn't contain a right operand");
                };
                assert_eq!(right.as_rule(), Rule::Symbol);
            }
        }
    }

    #[test]
    fn parse_logical_and_success() {
        let data = [
            "100 && 150.12",
            "100 && -150.12",
            "-100 && -150.12",
            "-100 && 150.12",
            "100&& 150.12",
            "100 &&-150.12",
            "-100&&-150.12",
            "100 && true",
            "true&& true",
            "true &&false",
            "false&&false",
            "\"hello\" && 100",
            "\"hello\" && -100",
            "\"hello\" && true",
            "\"hello\" && false",
            "\"hello\" && customer.name.length",
            "\"hello\" && customer.name().length()",
            "\"hello\" && customer().name().length()",
            "\"hello\" && customer().name23_23().length()",
            "\"hello\" && \"hello world\"",
            "100 && \"hello\"",
            "-100 && \"hello\"",
            "true && \"hello\"",
            "false && \"hello\"",
            "customer.name.length && \"hello\"",
            "\"hello world\" && \"hello\"",
        ];
        for op in data {
            let Ok(pairs) = ExprParser::parse(Rule::LogicalExpression, op) else {
                panic!("unable to parse LogicalExpression rule");
            };

            for pair in pairs {
                let mut pair_inner = pair.into_inner();

                let Some(left) = pair_inner.next() else {
                    panic!("parsed value doesn't contain a left operand");
                };
                assert_eq!(left.as_rule(), Rule::Expression);

                let Some(operator) = pair_inner.next() else {
                    panic!("parsed value doesn't contain an operator");
                };
                assert_eq!(operator.as_rule(), Rule::AndOperator);

                let Some(right) = pair_inner.next() else {
                    panic!("parsed value doesn't contain a right operand");
                };
                assert_eq!(right.as_rule(), Rule::Expression);
            }
        }
    }

    #[test]
    fn parse_logical_or_success() {
        let data = [
            "100 || 150.12",
            "100 || -150.12",
            "-100 || -150.12",
            "-100 || 150.12",
            "100|| 150.12",
            "100 ||-150.12",
            "-100||-150.12",
            "100 || true",
            "true|| true",
            "true ||false",
            "false||false",
            "\"hello\" || 100",
            "\"hello\" || -100",
            "\"hello\" || true",
            "\"hello\" || false",
            "\"hello\" || customer.name.length",
            "\"hello\" || customer.name().length()",
            "\"hello\" || customer().name().length()",
            "\"hello\" || customer().name23_23().length()",
            "\"hello\" || \"hello world\"",
            "100 || \"hello\"",
            "-100 || \"hello\"",
            "true || \"hello\"",
            "false || \"hello\"",
            "customer.name.length || \"hello\"",
            "\"hello world\" || \"hello\"",
        ];
        for op in data {
            let Ok(pairs) = ExprParser::parse(Rule::LogicalExpression, op) else {
                panic!("unable to parse LogicalExpression rule");
            };

            for pair in pairs {
                let mut pair_inner = pair.into_inner();

                let Some(left) = pair_inner.next() else {
                    panic!("parsed value doesn't contain a left operand");
                };
                assert_eq!(left.as_rule(), Rule::Expression);

                let Some(operator) = pair_inner.next() else {
                    panic!("parsed value doesn't contain an operator");
                };
                assert_eq!(operator.as_rule(), Rule::OrOperator);

                let Some(right) = pair_inner.next() else {
                    panic!("parsed value doesn't contain a right operand");
                };
                assert_eq!(right.as_rule(), Rule::Expression);
            }
        }
    }

    #[test]
    fn parse_composite_logical_expression_success() {
        let data = ["100 == 200 && true == false || (\"hello world\" > \"world\")"];
        for expr in data {
            let Ok(pairs) = ExprParser::parse(Rule::LogicalExpression, expr) else {
                panic!("unable to parse LogicalExpression rule");
            };
            // for pair in pairs {
            //     let mut pair_inner = pair.into_inner();
            // }
            dbg!(&pairs);
            panic!();
        }
    }

    #[test]
    fn parse_full_expression_success() {
        let data = ["100 == 200 && true == false", "100 == 200"];
        for expr in data {
            let pair = ExprParser::parse(Rule::Full, expr);
            dbg!(&pair);
            assert!(pair.is_ok());
        }
    }
}
