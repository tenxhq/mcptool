#![allow(dead_code)]

use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use textwrap::{wrap, Options};
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
    color: bool,
    width: usize,
    indent: usize,
}

impl Output {
    pub fn new(color: bool, width: usize) -> Self {
        let color_choice = if color {
            ColorChoice::Always
        } else {
            ColorChoice::Never
        };

        Self {
            stdout: Arc::new(Mutex::new(StandardStream::stdout(color_choice))),
            json: false,
            color,
            width,
            indent: 0,
        }
    }

    /// Output JSON with syntax highlighting if color is enabled
    fn output_json(&self, json_str: &str) -> io::Result<()> {
        if self.color {
            // Load syntax highlighting assets
            let ps = SyntaxSet::load_defaults_newlines();
            let ts = ThemeSet::load_defaults();

            let syntax = ps.find_syntax_by_extension("json").unwrap();
            let theme = &ts.themes["Solarized (dark)"];
            let mut h = HighlightLines::new(syntax, theme);
            let mut stdout = self.stdout.lock().unwrap();
            for line in LinesWithEndings::from(json_str) {
                let ranges: Vec<(Style, &str)> = h.highlight_line(line, &ps).unwrap();
                let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
                write!(stdout, "{escaped}")?;
            }
            stdout.reset()?;
            stdout.flush()
        } else {
            self.raw(json_str)
        }
    }

    /// Output a JSON value with syntax highlighting if appropriate
    pub fn json_value<T: serde::Serialize>(&self, value: &T) -> Result<()> {
        let json_str = serde_json::to_string_pretty(value)?;
        self.output_json(&json_str)?;
        Ok(())
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

    /// Return a copy of this Output with indent incremented by 4 spaces.
    pub fn indent(&self) -> Self {
        let mut output = self.clone();
        output.indent += 4;
        output
    }

    /// Helper method to write a line with proper indentation and text wrapping.
    fn write_block(&self, message: &str) -> io::Result<()> {
        let mut stdout = self.stdout.lock().unwrap();
        let indent_str = " ".repeat(self.indent);

        // Calculate available width for text (total width minus indent)
        let available_width = self.width.saturating_sub(self.indent);

        // If available width is too small, just use basic wrapping
        if available_width < 10 {
            writeln!(stdout, "{indent_str}{message}")
        } else {
            // Use textwrap to wrap the text properly
            let options = Options::new(available_width)
                .initial_indent("")
                .subsequent_indent("");

            let wrapped_lines = wrap(message, &options);

            for line in wrapped_lines {
                writeln!(stdout, "{indent_str}{line}")?;
            }
            Ok(())
        }
    }

    /// Helper method to write a line with proper indentation, text wrapping, and color.
    fn write_block_with_color(&self, message: &str, color_spec: &ColorSpec) -> io::Result<()> {
        let mut stdout = self.stdout.lock().unwrap();
        let indent_str = " ".repeat(self.indent);

        // Calculate available width for text (total width minus indent)
        let available_width = self.width.saturating_sub(self.indent);

        stdout.set_color(color_spec)?;

        // If available width is too small, just use basic wrapping
        if available_width < 10 {
            writeln!(stdout, "{indent_str}{message}")?;
        } else {
            // Use textwrap to wrap the text properly
            let options = Options::new(available_width)
                .initial_indent("")
                .subsequent_indent("");

            let wrapped_lines = wrap(message, &options);

            for line in wrapped_lines {
                writeln!(stdout, "{indent_str}{line}")?;
            }
        }

        stdout.reset()?;
        stdout.flush()
    }

    /// Raw output that is not affected by output settings
    fn raw(&self, message: impl Into<String>) -> io::Result<()> {
        let message = message.into();
        self.write_block(&message)
    }

    pub fn text(&self, message: impl Into<String>) -> io::Result<()> {
        if self.json {
            return Ok(());
        }

        let message = message.into();
        self.write_block(&message)
    }

    pub fn h1(&self, message: impl Into<String>) -> io::Result<()> {
        if self.json {
            return Ok(());
        }

        let message = message.into();

        // Create left-aligned header with padding to fill the FULL width (including indent)
        // The header background should span the entire terminal width
        let message_with_spaces = format!(" {message} ");
        let indent_str = " ".repeat(self.indent);
        let total_content_length = self.indent + message_with_spaces.len();
        let padding = self.width.saturating_sub(total_content_length);
        let header = format!(
            "{}{}{}",
            indent_str,
            message_with_spaces,
            " ".repeat(padding)
        );

        // Set lighter content text on dark background for better readability
        let color_spec = ColorSpec::new()
            .set_fg(Some(SOLARIZED_BASE1))
            .set_bg(Some(SOLARIZED_BASE02))
            .set_bold(true)
            .clone();

        // Write directly to stdout with color, bypassing write_block to avoid double indentation
        let mut stdout = self.stdout.lock().unwrap();
        stdout.set_color(&color_spec)?;
        writeln!(stdout, "{header}")?;
        stdout.reset()?;
        stdout.flush()
    }

    pub fn h2(&self, message: impl Into<String>) -> io::Result<()> {
        if self.json {
            return Ok(());
        }

        let message = message.into();

        // Use highlighted foreground color without background
        let color_spec = ColorSpec::new()
            .set_fg(Some(SOLARIZED_BLUE))
            .set_bold(true)
            .clone();

        self.write_block_with_color(&message, &color_spec)
    }

    pub fn h3(&self, message: impl Into<String>) -> io::Result<()> {
        if self.json {
            return Ok(());
        }

        let message = message.into();

        // Just bold text, no color change
        let color_spec = ColorSpec::new().set_bold(true).clone();

        self.write_block_with_color(&message, &color_spec)
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
        let formatted_message = format!("{prefix} {message}");

        let mut color_spec = ColorSpec::new();
        color_spec.set_fg(Some(color));
        if bold {
            color_spec.set_bold(true);
        }

        self.write_block_with_color(&formatted_message, &color_spec)
    }

    pub fn trace(&self, message: impl Into<String>, level: Level) -> io::Result<()> {
        let message = message.into();
        let formatted_message = format!("trace: {message}");

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

        self.write_block_with_color(&formatted_message, &color_spec)
    }

    pub fn list_tools_result(
        &self,
        tools_result: &tenx_mcp::schema::ListToolsResult,
    ) -> Result<()> {
        if self.json {
            // Output as JSON
            let json = serde_json::to_string_pretty(tools_result)?;
            self.output_json(&json)?;
        } else {
            // Output as formatted text
            if tools_result.tools.is_empty() {
                self.text("No tools.")?;
            } else {
                self.h1("Tools")?;

                let out = self.indent();
                for tool in &tools_result.tools {
                    out.h2(&tool.name)?;

                    // Description
                    if let Some(description) = &tool.description {
                        let out = out.indent();
                        for line in description.lines() {
                            out.text(line)?;
                        }
                    }
                    out.text("")?;

                    // Annotations
                    if let Some(annotations) = &tool.annotations {
                        let out = out.indent();
                        out.h2("Annotations")?;
                        let out = out.indent();
                        if let Some(title) = &annotations.title {
                            out.text(format!("title: {title}"))?;
                        }
                    }

                    // Input arguments
                    if let Some(properties) = &tool.input_schema.properties {
                        if !properties.is_empty() {
                            let out = out.indent();
                            out.h2("Input")?;
                            let out = out.indent();
                            out.toolschema(&tool.input_schema)?;
                        }
                    }

                    // Output schema
                    if let Some(output_schema) = &tool.output_schema {
                        if let Some(properties) = &output_schema.properties {
                            if !properties.is_empty() {
                                let out = out.indent();
                                out.h2("Output")?;
                                let out = out.indent();
                                out.toolschema(output_schema)?;
                            }
                        }
                    }

                    out.text("")?; // Extra blank line between tools
                }
            }
        }
        Ok(())
    }

    pub fn toolschema(&self, schema: &tenx_mcp::schema::ToolSchema) -> Result<()> {
        if let Some(properties) = &schema.properties {
            if !properties.is_empty() {
                // Sort properties to show required ones first
                let mut sorted_props: Vec<_> = properties.iter().collect();
                sorted_props.sort_by(|(a_name, _), (b_name, _)| {
                    let a_required = schema.is_required(a_name);
                    let b_required = schema.is_required(b_name);

                    // Required fields come first
                    b_required.cmp(&a_required).then(a_name.cmp(b_name))
                });

                for (name, prop_schema) in sorted_props {
                    let is_required = schema.is_required(name);

                    // Extract type from schema
                    let type_str = if let Some(serde_json::Value::String(t)) =
                        prop_schema.get("type")
                    {
                        t.to_string()
                    } else if let Some(serde_json::Value::Array(types)) = prop_schema.get("type") {
                        // Handle union types like ["string", "null"]
                        types
                            .iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(" | ")
                    } else {
                        "unknown".to_string()
                    };

                    let required_text = if is_required { " (required)" } else { "" };
                    self.h3(format!("{name}: {type_str}{required_text}"))?;

                    // Show schema details indented further
                    let out = self.indent();

                    // Make a mutable copy of the schema
                    let mut schema_copy = prop_schema.clone();

                    // Remove type since we already displayed it
                    if let Some(obj) = schema_copy.as_object_mut() {
                        obj.remove("type");

                        // Extract and display description if it exists
                        if let Some(serde_json::Value::String(desc)) = obj.remove("description") {
                            out.text(&desc)?;
                        }

                        // If there are remaining properties, display them as JSON
                        if !obj.is_empty() {
                            let rendered_schema = serde_json::to_string_pretty(&schema_copy)?;
                            for line in rendered_schema.lines() {
                                out.text(line)?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn ping(&self) -> Result<()> {
        if self.json {
            self.output_json("{}")?;
        } else {
            self.success("Ping successful!")?;
        }
        Ok(())
    }
}

impl Default for Output {
    fn default() -> Self {
        // Default to color detection based on TTY
        let color = atty::is(atty::Stream::Stdout);
        Self::new(color, 80)
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
