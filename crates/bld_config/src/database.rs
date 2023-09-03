use crate::definitions::LOCAL_DB;
use serde_derive::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BldDatabaseConfig {
    Legacy(String),
	Connection {
		engine: String,
		connection_string: String,
	}
}

impl Default for BldDatabaseConfig {
	fn default() -> Self {
		Self::Legacy(LOCAL_DB.to_string())
	}
}
