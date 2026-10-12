use std::collections::HashMap;

pub fn parse_variables<I, S>(variables: I) -> HashMap<String, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    variables
        .into_iter()
        .flat_map(|v| {
            let (name, value) = v.as_ref().split_once('=')?;
            Some((name.to_owned(), value.to_owned()))
        })
        .collect::<HashMap<String, String>>()
}

#[cfg(test)]
mod tests {
    use crate::variables::parse_variables;

    #[test]
    pub fn parse_variables_success() {
        let data = vec![
            ("name=john", "name", "john"),
            ("surname=doe", "surname", "doe"),
            ("age=30", "age", "30"),
        ];
        let expr: Vec<String> = data.iter().map(|x| x.0.to_string()).collect();
        let variables = parse_variables(&expr);
        for (_expr, name, expected) in data {
            let actual = variables.get(name);
            assert!(actual.is_some());
            assert_eq!(actual.unwrap(), expected);
        }
    }

    #[test]
    pub fn parse_variables_from_str_iter_success() {
        let data = vec![
            ("name=john", "name", "john"),
            ("surname=doe", "surname", "doe"),
            ("age=30", "age", "30"),
        ];
        let variables = parse_variables(data.iter().map(|x| x.0));
        for (_expr, name, expected) in data {
            let actual = variables.get(name);
            assert!(actual.is_some());
            assert_eq!(actual.unwrap(), expected);
        }
    }

    #[test]
    pub fn parse_variables_value_with_multiple_equals_signs() {
        let data = vec![
            ("token=YWJj==", "token", "YWJj=="),
            ("url=https://x.io/?a=1&b=2", "url", "https://x.io/?a=1&b=2"),
        ];
        let expr: Vec<String> = data.iter().map(|x| x.0.to_string()).collect();
        let variables = parse_variables(&expr);
        for (_expr, name, expected) in data {
            let actual = variables.get(name);
            assert!(actual.is_some());
            assert_eq!(actual.unwrap(), expected);
        }
    }
}
