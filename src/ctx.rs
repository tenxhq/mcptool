use crate::storage::{StorageError, TokenStorage};

use std::path::PathBuf;

pub const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "-",
    env!("VERGEN_GIT_SHA"),
    " (",
    env!("VERGEN_BUILD_DATE"),
    ")"
);

/// Central context passed to all operatoins
pub struct Ctx {
    /// Path to the configuration directory
    pub config_path: PathBuf,
}

impl Ctx {
    /// Create a new MCPTool instance with the given configuration path
    pub fn new(config_path: PathBuf) -> Self {
        Self { config_path }
    }

    /// Create a TokenStorage instance using the configured path
    pub fn storage(&self) -> Result<TokenStorage, StorageError> {
        TokenStorage::new(self.config_path.clone())
    }
}
