#![allow(dead_code)]

use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use terminal_size::{terminal_size, Width};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::Result;

/// Log level configuration for the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    /// Convert to tracing Level
    pub fn to_tracing_level(&self) -> Level {
        match self {
            LogLevel::Error => Level::ERROR,
            LogLevel::Warn => Level::WARN,
            LogLevel::Info => Level::INFO,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Trace => Level::TRACE,
        }
    }

    /// Get the level as a string for env filter
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }
    }
}

impl std::str::FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "error" => Ok(LogLevel::Error),
            "warn" => Ok(LogLevel::Warn),
            "info" => Ok(LogLevel::Info),
            "debug" => Ok(LogLevel::Debug),
            "trace" => Ok(LogLevel::Trace),
            _ => Err(format!("Invalid log level: {s}")),
        }
    }
}

// Solarized Dark color scheme constants
// Background tones
const SOLARIZED_BASE03: Color = Color::Rgb(0, 43, 54); // darkest background
const SOLARIZED_BASE02: Color = Color::Rgb(7, 54, 66); // dark background
const SOLARIZED_BASE01: Color = Color::Rgb(88, 110, 117); // darker content
const SOLARIZED_BASE00: Color = Color::Rgb(101, 123, 131); // dark content

// Content tones
const SOLARIZED_BASE0: Color = Color::Rgb(131, 148, 150); // light content
const SOLARIZED_BASE1: Color = Color::Rgb(147, 161, 161); // lighter content
const SOLARIZED_BASE2: Color = Color::Rgb(238, 232, 213); // light background
const SOLARIZED_BASE3: Color = Color::Rgb(253, 246, 227); // lightest background

// Accent colors
const SOLARIZED_YELLOW: Color = Color::Rgb(181, 137, 0);
const SOLARIZED_ORANGE: Color = Color::Rgb(203, 75, 22);
const SOLARIZED_RED: Color = Color::Rgb(220, 50, 47);
const SOLARIZED_MAGENTA: Color = Color::Rgb(211, 54, 130);
const SOLARIZED_VIOLET: Color = Color::Rgb(108, 113, 196);
const SOLARIZED_BLUE: Color = Color::Rgb(38, 139, 210);
const SOLARIZED_CYAN: Color = Color::Rgb(42, 161, 152);
const SOLARIZED_GREEN: Color = Color::Rgb(133, 153, 0);

/// Handles all output formatting for the application.
///
/// This struct provides a unified interface for outputting text to the console,
/// with support for both human-readable formatted output and machine-readable JSON output.
/// It uses the Solarized Dark color scheme for styled terminal output and can switch
/// between colored text mode and JSON mode based on the `json` flag.
///
/// The struct is `Clone` and thread-safe, allowing it to be shared across different
/// parts of the application.
#[derive(Clone)]
pub struct Output {
    stdout: Arc<Mutex<StandardStream>>,
    json: bool,
}

impl Output {
    pub fn new(json: bool) -> Self {
        Self {
            stdout: Arc::new(Mutex::new(StandardStream::stdout(ColorChoice::Auto))),
            json,
        }
    }

    /// Set JSON output mode.
    pub fn with_json(mut self, json: bool) -> Self {
        self.json = json;
        self
    }

    /// Enable logging with the specified log level and return self.
    pub fn with_logging(self, level: Option<LogLevel>) -> Result<Self> {
        if let Some(log_level) = level {
            let env_filter = EnvFilter::try_new(log_level.as_str()).unwrap_or_default();
            let output_layer = OutputLayer::new(self.clone());

            tracing_subscriber::registry()
                .with(env_filter)
                .with(output_layer)
                .init();
        }

        Ok(self)
    }

    /// Raw output that is not affected by output settings
    fn raw(&self, message: impl Into<String>) -> io::Result<()> {
        let message = message.into();
        let mut stdout = self.stdout.lock().unwrap();
        writeln!(stdout, "{message}")
    }

    pub fn text(&self, message: impl Into<String>) -> io::Result<()> {
        if self.json {
            return Ok(());
        }

        let message = message.into();
        let mut stdout = self.stdout.lock().unwrap();
        writeln!(stdout, "{message}")
    }

    pub fn heading(&self, message: impl Into<String>) -> io::Result<()> {
        if self.json {
            return Ok(());
        }

        let message = message.into();
        let mut stdout = self.stdout.lock().unwrap();

        // Get terminal width, default to 80 if unable to detect
        let width = if let Some((Width(w), _)) = terminal_size() {
            w as usize
        } else {
            80
        };

        // Calculate padding needed
        let message_with_spaces = format!(" {message} ");
        let padding = width.saturating_sub(message_with_spaces.len());
        let left_pad = padding / 2;
        let right_pad = padding - left_pad;

        // Create the full-width header
        let header = format!(
            "{}{}{}",
            " ".repeat(left_pad),
            message_with_spaces,
            " ".repeat(right_pad)
        );

        // Set lighter content text on dark background for better readability
        stdout.set_color(
            ColorSpec::new()
                .set_fg(Some(SOLARIZED_BASE1))
                .set_bg(Some(SOLARIZED_BASE02))
                .set_bold(true),
        )?;
        write!(stdout, "{header}")?;
        stdout.reset()?;
        writeln!(stdout)?;
        stdout.flush()
    }

    pub fn warn(&self, message: impl Into<String>) -> io::Result<()> {
        self.status(message, "[WARNING]", SOLARIZED_YELLOW, false)
    }

    pub fn error(&self, message: impl Into<String>) -> io::Result<()> {
        self.status(message, "[ERROR]", SOLARIZED_RED, true)
    }

    pub fn success(&self, message: impl Into<String>) -> io::Result<()> {
        self.status(message, "[OK]", SOLARIZED_GREEN, false)
    }

    pub fn debug(&self, message: impl Into<String>) -> io::Result<()> {
        self.status(message, "[DEBUG]", SOLARIZED_MAGENTA, false)
    }

    // Helper method to reduce repetition
    fn status(
        &self,
        message: impl Into<String>,
        prefix: &str,
        color: Color,
        bold: bool,
    ) -> io::Result<()> {
        if self.json {
            return Ok(());
        }

        let message = message.into();
        let mut stdout = self.stdout.lock().unwrap();

        let mut color_spec = ColorSpec::new();
        color_spec.set_fg(Some(color));
        if bold {
            color_spec.set_bold(true);
        }

        stdout.set_color(&color_spec)?;
        writeln!(stdout, "{prefix} {message}")?;
        stdout.reset()?;
        stdout.flush()
    }

    pub fn trace(&self, message: impl Into<String>, level: Level) -> io::Result<()> {
        let message = message.into();
        let mut stdout = self.stdout.lock().unwrap();

        let mut color_spec = ColorSpec::new();
        match level {
            Level::ERROR => {
                color_spec.set_fg(Some(SOLARIZED_RED)).set_bold(true);
            }
            Level::WARN => {
                color_spec.set_fg(Some(SOLARIZED_YELLOW));
            }
            Level::INFO => {
                color_spec.set_fg(Some(SOLARIZED_BASE0)); // Light content color
            }
            Level::DEBUG => {
                color_spec.set_fg(Some(SOLARIZED_MAGENTA));
            }
            Level::TRACE => {
                color_spec.set_fg(Some(SOLARIZED_BASE01)); // Darker content for trace
            }
        };

        stdout.set_color(&color_spec)?;
        writeln!(stdout, "trace: {message}")?;
        stdout.reset()?;
        stdout.flush()
    }

    pub fn list_tools_result(
        &self,
        tools_result: &tenx_mcp::schema::ListToolsResult,
    ) -> Result<()> {
        if self.json {
            // Output as JSON
            let json = serde_json::to_string_pretty(tools_result)?;
            self.raw(json)?;
        } else {
            // Output as formatted text
            if tools_result.tools.is_empty() {
                self.text("No tools available from this server.")?;
            } else {
                self.heading(format!("Available tools ({}):", tools_result.tools.len()))?;
                self.text("")?;
                for tool in &tools_result.tools {
                    self.text(format!("  - {}", tool.name))?;

                    self.text("")?;
                    self.text("    Description:")?;
                    self.text("")?;
                    match &tool.description {
                        Some(description) => {
                            for line in description.lines() {
                                self.text(format!("      {line}"))?;
                            }
                        }
                        None => self.text("      No description available")?,
                    }

                    self.text("")?;
                    self.text("    Annotations:")?;
                    self.text("")?;
                    match &tool.annotations {
                        Some(annotations) => {
                            self.text(format!("      {:?}", annotations.title))?;
                        }
                        None => self.text("      No annotations available")?,
                    }

                    self.text("")?;
                    self.text("    Input arguments:")?;
                    self.text("")?;

                    // TODO Show required inputs first?
                    match &tool.input_schema.properties {
                        Some(properties) => {
                            for (name, schema) in properties {
                                let rendered_schema = serde_json::to_string_pretty(schema)?;
                                let is_required = &tool
                                    .clone()
                                    .input_schema
                                    .required
                                    .is_some_and(|list| list.contains(name));
                                self.text(format!("      {name} - (required: {is_required})"))?;
                                self.text("")?;

                                for line in rendered_schema.lines() {
                                    self.text(format!("        {line}"))?;
                                }
                                self.text("")?;
                            }
                        }
                        None => self.text("      No input schema available")?,
                    }

                    self.text("")?; // Extra blank line between tools
                }
            }
        }
        Ok(())
    }

    pub fn ping(&self) -> Result<()> {
        if self.json {
            self.text("{}")?;
        } else {
            self.success("Ping successful!")?;
        }
        Ok(())
    }
}

impl Default for Output {
    fn default() -> Self {
        Self::new(false)
    }
}

/// A tracing subscriber layer that forwards log messages to an Output instance.
///
/// This struct implements the `tracing_subscriber::Layer` trait to integrate with
/// the tracing ecosystem. It captures log events and forwards them to the Output
/// struct for consistent formatting. This allows application logs to respect the
/// same formatting rules (including JSON mode) as regular output.
pub struct OutputLayer {
    output: Output,
}

impl OutputLayer {
    pub fn new(output: Output) -> Self {
        Self { output }
    }
}

impl<S> Layer<S> for OutputLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let level = event.metadata().level();
        let mut visitor = MessageVisitor {
            message: String::new(),
        };
        event.record(&mut visitor);
        let message = visitor.message;

        let _ = self.output.trace(&message, *level);
    }
}

/// A visitor for extracting message content from tracing events.
///
/// This struct implements the `tracing::field::Visit` trait to extract the message
/// field from tracing events. It's used internally by `OutputLayer` to get the
/// actual log message text that needs to be formatted and displayed.
struct MessageVisitor {
    message: String,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }
}
