use serde::{Deserialize, Serialize};

use crate::definitions::LOCAL_PACKAGES_CACHE;

#[derive(Debug, Serialize, Deserialize)]
pub struct BldPackages {
    #[serde(default = "BldPackages::default_cache")]
    pub cache: String,
}

impl BldPackages {
    fn default_cache() -> String {
        format!("~/{}", LOCAL_PACKAGES_CACHE)
    }
}

impl Default for BldPackages {
    fn default() -> Self {
        Self {
            cache: Self::default_cache(),
        }
    }
}
