use std::path::PathBuf;

use crate::{
    output::{LogLevel, Output},
    storage::TokenStorage,
    Result,
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
        logs: Option<LogLevel>,
        json: bool,
        color: bool,
        width: usize,
    ) -> Result<Self> {
        let output = Output::new(color, width)
            .with_json(json)
            .with_logging(logs)?;

        Ok(Self {
            config_path,
            output,
        })
    }

    /// Create a TokenStorage instance using the configured path
    pub fn storage(&self) -> Result<TokenStorage> {
        Ok(TokenStorage::new(self.config_path.clone())?)
    }
}
