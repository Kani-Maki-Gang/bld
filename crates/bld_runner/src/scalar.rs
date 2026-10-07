use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ScalarValue {
    Number(f64),
    Boolean(bool),
    Text(String),
}

#[cfg(all(test, feature = "all"))]
mod tests {
    use super::ScalarValue;

    #[test]
    pub fn scalar_value_mixed_list_deserialize_success() {
        let yaml = "[1, 2.5, true, linux, \"1\"]";
        let values: Vec<ScalarValue> = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(
            values,
            vec![
                ScalarValue::Number(1.0),
                ScalarValue::Number(2.5),
                ScalarValue::Boolean(true),
                ScalarValue::Text("linux".to_string()),
                ScalarValue::Text("1".to_string()),
            ]
        );
    }

    #[test]
    pub fn scalar_value_null_deserialize_failure() {
        let result: Result<ScalarValue, _> = serde_yaml_ng::from_str("~");
        assert!(result.is_err());
    }
}
