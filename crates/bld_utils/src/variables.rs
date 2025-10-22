use std::collections::HashMap;

pub fn parse_variables(variables: &[String]) -> HashMap<String, String> {
    variables
        .iter()
        .flat_map(|v| {
            let mut split = v.split('=');
            let name = split.next()?.to_owned();
            let value = split.next()?.to_owned();
            Some((name, value))
        })
        .collect::<HashMap<String, String>>()
}

pub fn parse_variables_iter<'a>(
    variables: impl Iterator<Item = &'a str>,
) -> HashMap<String, String> {
    variables
        .flat_map(|v| {
            let mut split = v.split('=');
            let name = split.next()?.to_owned();
            let value = split.next()?.to_owned();
            Some((name, value))
        })
        .collect::<HashMap<String, String>>()
}

#[cfg(test)]
mod tests {
    use crate::variables::{parse_variables, parse_variables_iter};

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
    pub fn parse_variables_iter_success() {
        let data = vec![
            ("name=john", "name", "john"),
            ("surname=doe", "surname", "doe"),
            ("age=30", "age", "30"),
        ];
        let variables = parse_variables_iter(data.iter().map(|x| x.0));
        for (_expr, name, expected) in data {
            let actual = variables.get(name);
            assert!(actual.is_some());
            assert_eq!(actual.unwrap(), expected);
        }
    }
}
