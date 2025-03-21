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
                panic!("unable to parse EQUALS operator");
            };
            for pair in pairs {
                match pair.as_rule() {
                    Rule::EqualsOperator => {
                        assert_eq!(pair.as_span().get_input(), op);
                    }
                    _ => panic!("parsed value is not a EQUALS operator rule"),
                }
            }
        }
    }

    #[test]
    fn parse_greater_operator_success() {
        let data = [">", " >", "> ", " > "];
        for op in data {
            let Ok(pairs) = ExprParser::parse(Rule::GreaterOperator, op) else {
                panic!("unable to parse GREATER operator");
            };
            for pair in pairs {
                match pair.as_rule() {
                    Rule::GreaterOperator => {
                        assert_eq!(pair.as_span().get_input(), op);
                    }
                    _ => panic!("parsed value is not a GREATER operator rule"),
                }
            }
        }
    }

    #[test]
    fn parse_greater_equals_operator_success() {
        let data = [">=", " >=", ">= ", " >= "];
        for op in data {
            let Ok(pairs) = ExprParser::parse(Rule::GreaterEqualsOperator, op) else {
                panic!("unable to parse GREATER EQUALS operator");
            };
            for pair in pairs {
                match pair.as_rule() {
                    Rule::GreaterEqualsOperator => {
                        assert_eq!(pair.as_span().get_input(), op);
                    }
                    _ => panic!("parsed value is not a GREATER EQUALS operator rule"),
                }
            }
        }
    }

    #[test]
    fn parse_less_operator_success() {
        let data = ["<", " <", "< ", " < "];
        for op in data {
            let Ok(pairs) = ExprParser::parse(Rule::LessOperator, op) else {
                panic!("unable to parse LESS operator");
            };
            for pair in pairs {
                match pair.as_rule() {
                    Rule::LessOperator => {
                        assert_eq!(pair.as_span().get_input(), op);
                    }
                    _ => panic!("parsed value is not a LESS operator rule"),
                }
            }
        }
    }

    #[test]
    fn parse_less_equals_operator_success() {
        let data = ["<=", " <=", "<= ", " <= "];
        for op in data {
            let Ok(pairs) = ExprParser::parse(Rule::LessEqualsOperator, op) else {
                panic!("unable to parse LESS EQUALS operator");
            };
            for pair in pairs {
                match pair.as_rule() {
                    Rule::LessEqualsOperator => {
                        assert_eq!(pair.as_span().get_input(), op);
                    }
                    _ => panic!("parsed value is not a LESS EQUALS operator rule"),
                }
            }
        }
    }

    #[test]
    fn parse_and_operator_success() {
        let data = ["&&", " &&", "&& ", " && "];
        for op in data {
            let Ok(pairs) = ExprParser::parse(Rule::AndOperator, op) else {
                panic!("unable to parse AND operator");
            };
            for pair in pairs {
                match pair.as_rule() {
                    Rule::AndOperator => {
                        assert_eq!(pair.as_span().get_input(), op);
                    }
                    _ => panic!("parsed value is not an AND operator rule"),
                }
            }
        }
    }

    #[test]
    fn parse_or_operator_success() {
        let data = ["||", " ||", "|| ", " || "];
        for op in data {
            let Ok(pairs) = ExprParser::parse(Rule::OrOperator, op) else {
                panic!("unable to parse OR operator");
            };
            for pair in pairs {
                match pair.as_rule() {
                    Rule::OrOperator => {
                        assert_eq!(pair.as_span().get_input(), op);
                    }
                    _ => panic!("parsed value is not an OR operator rule"),
                }
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
                panic!("unable to parse NUMBER symbol");
            };
            for pair in pairs {
                match pair.as_rule() {
                    Rule::Number => {
                        assert_eq!(pair.as_span().get_input(), number);
                    }
                    _ => panic!("parsed value is not a NUMBER rule"),
                }
            }
        }
    }

    #[test]
    fn parse_bool_success() {
        let data = ["true", "false"];
        for boolean in data {
            let Ok(pairs) = ExprParser::parse(Rule::Boolean, boolean) else {
                panic!("unable to parse BOOLEAN symbol");
            };
            for pair in pairs {
                match pair.as_rule() {
                    Rule::Boolean => {
                        assert_eq!(pair.as_span().get_input(), boolean);
                    }
                    _ => panic!("parse value is not a BOOLEAN rule"),
                }
            }
        }
    }

    #[test]
    fn parse_string_success() {
        let data = ["\"hellow world\""];
        for string in data {
            let Ok(pairs) = ExprParser::parse(Rule::String, string) else {
                panic!("unable to parse STRING symbol");
            };
            for pair in pairs {
                match pair.as_rule() {
                    Rule::String => {
                        assert_eq!(pair.as_span().get_input(), string);
                    }
                    _ => panic!("parsed value is not a STRING rule"),
                }
            }
        }
    }

    #[test]
    fn parse_object_success() {
        let data = [
            "customer",
            "customer.name",
            "customer.123",
            "customer.name.toString()",
            "customer.name.toString()",
            "customer().name().length()",
            "customer().name23_23().length()",
        ];
        for (i, object) in data.iter().enumerate() {
            let Ok(pairs) = ExprParser::parse(Rule::Object, object) else {
                panic!("unable to parse OBJECT symbol");
            };
            if i == 3 {
                dbg!(&pairs);
                panic!("stop");
            }
            for pair in pairs {
                match pair.as_rule() {
                    Rule::Object => {
                        assert_eq!(pair.as_span().get_input(), *object);
                    }
                    _ => panic!("parsed value is not a OBJECT rule"),
                }
            }
        }
    }

    #[test]
    fn parse_symbol_success() {
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
            "true",
            "false",
            "\"hello world\"",
            "customer",
            "customer.name",
            "customer.123",
            "customer.name.toString()",
            "customer().name().length()",
            "customer().name23_23().length()",
        ];
        for symbol in data {
            let Ok(pairs) = ExprParser::parse(Rule::Symbol, symbol) else {
                panic!("unable to parse SYMBOL");
            };
            for pair in pairs {
                match pair.as_rule() {
                    Rule::Symbol => {
                        for pair in pair.into_inner() {
                            match pair.as_rule() {
                                Rule::Number | Rule::Boolean | Rule::String | Rule::Object => {
                                    assert_eq!(pair.as_span().get_input(), symbol);
                                }
                                _ => panic!("parsed value is not a SYMBOL rule"),
                            }
                        }
                    }
                    _ => panic!("parsed value is not a SYMBOL rule"),
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
            let pair = ExprParser::parse(Rule::Greater, op);
            dbg!(&pair);
            assert!(pair.is_ok());
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
            let pair = ExprParser::parse(Rule::GreaterEquals, op);
            dbg!(&pair);
            assert!(pair.is_ok());
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
            let pair = ExprParser::parse(Rule::Less, op);
            dbg!(&pair);
            assert!(pair.is_ok());
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
            let pair = ExprParser::parse(Rule::LessEquals, op);
            dbg!(&pair);
            assert!(pair.is_ok());
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
            let pair = ExprParser::parse(Rule::LogicalExpression, op);
            dbg!(&pair);
            assert!(pair.is_ok());
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
            let pair = ExprParser::parse(Rule::LogicalExpression, op);
            dbg!(&pair);
            assert!(pair.is_ok());
        }
    }

    #[test]
    fn parse_composite_logical_expression_success() {
        let data = ["100 == 200 && true == false || (\"hello world\" > \"world\")"];
        for expr in data {
            let pair = ExprParser::parse(Rule::LogicalExpression, expr);
            dbg!(&pair);
            assert!(pair.is_ok());
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
