use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    #[default]
    Pipeline,
    Action,
}

impl FileType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pipeline => "pipeline",
            Self::Action => "action",
        }
    }
}

impl From<&str> for FileType {
    fn from(value: &str) -> Self {
        match value {
            "action" => Self::Action,
            _ => Self::Pipeline,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResponse {
    pub id: String,
    pub pipeline: String,
    #[serde(default)]
    pub file_type: FileType,
}
