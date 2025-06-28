use std::path::PathBuf;

use crate::{
    output::Output,
    storage::{StorageError, TokenStorage},
};

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
    /// Output instance for consistent formatting
    pub output: Output,
}

impl Ctx {
    /// Create a new MCPTool instance with the given configuration path
    pub fn new(
        config_path: PathBuf,
        logs: Option<Option<String>>,
        json: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let output = Output::new(false).with_json(json).with_logging(logs)?;

        Ok(Self {
            config_path,
            output,
        })
    }

    /// Create a TokenStorage instance using the configured path
    pub fn storage(&self) -> Result<TokenStorage, StorageError> {
        TokenStorage::new(self.config_path.clone())
    }
}
