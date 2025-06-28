use crate::storage::{StorageError, TokenStorage};

use std::path::PathBuf;

/// Central configuration and state management for mcptool
pub struct MCPTool {
    /// Path to the configuration directory
    pub config_path: PathBuf,
}

impl MCPTool {
    /// Create a new MCPTool instance with the given configuration path
    pub fn new(config_path: PathBuf) -> Self {
        Self { config_path }
    }

    /// Create a TokenStorage instance using the configured path
    pub fn storage(&self) -> Result<TokenStorage, StorageError> {
        TokenStorage::new(self.config_path.clone())
    }
}
