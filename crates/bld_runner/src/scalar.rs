use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ScalarValue {
    Number(f64),
    Boolean(bool),
    Text(String),
}
