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
