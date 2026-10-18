use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ScalarValue {
    Number(f64),
    Boolean(bool),
    Text(String),
}

impl ScalarValue {
    pub fn scalar_type(&self) -> ScalarType {
        match self {
            Self::Number(_) => ScalarType::Number,
            Self::Boolean(_) => ScalarType::Boolean,
            Self::Text(_) => ScalarType::Text,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ScalarType {
    Number,
    Boolean,
    Text,
    Array,
}

impl ScalarType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::Text => "text",
            Self::Array => "array",
        }
    }
}

pub fn single_scalar_type(items: &[ScalarValue]) -> Option<ScalarType> {
    let first = items.first()?.scalar_type();
    items
        .iter()
        .all(|x| x.scalar_type() == first)
        .then_some(first)
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
