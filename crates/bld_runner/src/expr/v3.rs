use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "expr.pest"]
struct ExpressionParser;

#[cfg(test)]
mod tests {
    use crate::Rule;

    use super::ExpressionParser;
    use pest::Parser;

    #[test]
    fn parse_equals_operator_success() {
        let data = ["==", " ==", "== ", " == "];
        for op in data {
            let pair = ExpressionParser::parse(Rule::EqualsOperator, op);
            dbg!(&pair);
            assert!(pair.is_ok());
        }
    }

    #[test]
    fn parse_greater_operator_success() {
        let data = [">", " >", "> ", " > "];
        for op in data {
            let pair = ExpressionParser::parse(Rule::GreaterOperator, op);
            dbg!(&pair);
            assert!(pair.is_ok());
        }
    }

    #[test]
    fn parse_greater_equals_operator_success() {
        let data = [">=", " >=", ">= ", " >= "];
        for op in data {
            let pair = ExpressionParser::parse(Rule::GreaterEqualsOperator, op);
            dbg!(&pair);
            assert!(pair.is_ok());
        }
    }

    #[test]
    fn parse_less_operator_success() {
        let data = ["<", " <", "< ", " < "];
        for op in data {
            let pair = ExpressionParser::parse(Rule::LessOperator, op);
            dbg!(&pair);
            assert!(pair.is_ok());
        }
    }

    #[test]
    fn parse_less_equals_operator_success() {
        let data = ["<=", " <=", "<= ", " <= "];
        for op in data {
            let pair = ExpressionParser::parse(Rule::LessEqualsOperator, op);
            dbg!(&pair);
            assert!(pair.is_ok());
        }
    }

    #[test]
    fn parse_and_operator_success() {
        let data = ["&&", " &&", "&& ", " && "];
        for op in data {
            let pair = ExpressionParser::parse(Rule::AndOperator, op);
            dbg!(&pair);
            assert!(pair.is_ok());
        }
    }

    #[test]
    fn parse_or_operator_success() {
        let data = ["||", " ||", "|| ", " || "];
        for op in data {
            let pair = ExpressionParser::parse(Rule::OrOperator, op);
            dbg!(&pair);
            assert!(pair.is_ok());
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
            let pair = ExpressionParser::parse(Rule::Number, number);
            dbg!(&pair);
            assert!(pair.is_ok());
        }
    }

    #[test]
    fn parse_bool_success() {
        let data = ["true", "false"];
        for boolean in data {
            let pair = ExpressionParser::parse(Rule::Boolean, boolean);
            dbg!(&pair);
            assert!(pair.is_ok());
        }
    }

    #[test]
    fn parse_string_success() {
        let data = ["\"hellow world\""];
        for string in data {
            let pair = ExpressionParser::parse(Rule::String, string);
            dbg!(&pair);
            assert!(pair.is_ok());
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
        for object in data {
            let pair = ExpressionParser::parse(Rule::Object, object);
            dbg!(&pair);
            assert!(pair.is_ok());
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
            let pair = ExpressionParser::parse(Rule::Symbol, symbol);
            dbg!(&pair);
            assert!(pair.is_ok());
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
            let pair = ExpressionParser::parse(Rule::Equals, op);
            dbg!(&pair);
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
            let pair = ExpressionParser::parse(Rule::Greater, op);
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
            let pair = ExpressionParser::parse(Rule::GreaterEquals, op);
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
            let pair = ExpressionParser::parse(Rule::Less, op);
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
            let pair = ExpressionParser::parse(Rule::LessEquals, op);
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
            let pair = ExpressionParser::parse(Rule::LogicalExpression, op);
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
            let pair = ExpressionParser::parse(Rule::LogicalExpression, op);
            dbg!(&pair);
            assert!(pair.is_ok());
        }
    }

    #[test]
    fn parse_composite_logical_expression_success() {
        let data = ["100 == 200 && true == false || (\"hello world\" > \"world\")"];
        for expr in data {
            let pair = ExpressionParser::parse(Rule::LogicalExpression, expr);
            dbg!(&pair);
            assert!(pair.is_ok());
        }
    }

    #[test]
    fn parse_full_expression_success() {
        let data = ["${{ 100 == 200 && true == false }}"];
        for expr in data {
            let pair = ExpressionParser::parse(Rule::Full, expr);
            dbg!(&pair);
            assert!(pair.is_ok());
        }
    }
}

