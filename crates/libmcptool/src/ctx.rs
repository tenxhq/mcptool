use std::path::PathBuf;

use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::{
    output::{Output, OutputLayer},
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
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let output = create_output_with_logging(logs)?;
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

pub fn init_logging(level: Option<Level>) -> Output {
    let output = Output::new();

    let env_filter = match level {
        Some(Level::ERROR) => EnvFilter::try_new("error").unwrap_or_default(),
        Some(Level::WARN) => EnvFilter::try_new("warn").unwrap_or_default(),
        Some(Level::INFO) => EnvFilter::try_new("info").unwrap_or_default(),
        Some(Level::DEBUG) => EnvFilter::try_new("debug").unwrap_or_default(),
        Some(Level::TRACE) => EnvFilter::try_new("trace").unwrap_or_default(),
        None => EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::try_new("info").unwrap()),
    };

    let output_layer = OutputLayer::new(output.clone());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(output_layer)
        .init();

    output
}

pub fn create_output_with_logging(
    logs: Option<Option<String>>,
) -> Result<Output, Box<dyn std::error::Error>> {
    if let Some(log_level) = logs {
        let level = match log_level.as_deref() {
            Some("error") => Some(Level::ERROR),
            Some("warn") => Some(Level::WARN),
            Some("info") => Some(Level::INFO),
            Some("debug") => Some(Level::DEBUG),
            Some("trace") => Some(Level::TRACE),
            Some(other) => {
                return Err(format!("Invalid log level: {other}").into());
            }
            None => Some(Level::INFO), // Default to INFO if --logs is used without a level
        };
        Ok(init_logging(level))
    } else {
        Ok(Output::new())
    }
}
