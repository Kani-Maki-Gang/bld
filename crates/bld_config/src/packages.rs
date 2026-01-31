use std::env::{home_dir, temp_dir};

use serde::{Deserialize, Serialize};

use crate::definitions::LOCAL_PACKAGES_CACHE;

#[derive(Debug, Serialize, Deserialize)]
pub struct BldPackages {
    #[serde(default = "BldPackages::default_cache")]
    pub cache: String,

    #[serde(default)]
    pub strict_sync: bool,
}

impl BldPackages {
    fn default_cache() -> String {
        let base_dir = home_dir().unwrap_or_else(temp_dir);
        format!("{}/{}", base_dir.to_string_lossy(), LOCAL_PACKAGES_CACHE)
    }
}

impl Default for BldPackages {
    fn default() -> Self {
        Self {
            cache: Self::default_cache(),
            strict_sync: false,
        }
    }
}
