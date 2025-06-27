#![allow(dead_code)]

use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use terminal_size::{Width, terminal_size};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

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

struct Inner {
    stdout: StandardStream,
}

#[derive(Clone)]
pub struct Output {
    inner: Arc<Mutex<Inner>>,
}

impl Output {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                stdout: StandardStream::stdout(ColorChoice::Auto),
            })),
        }
    }

    pub fn text(&self, message: &str) -> io::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner.stdout.reset()?;
        writeln!(inner.stdout, "{message}")?;
        inner.stdout.flush()
    }

    pub fn heading(&self, message: &str) -> io::Result<()> {
        let mut inner = self.inner.lock().unwrap();

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
        inner.stdout.set_color(
            ColorSpec::new()
                .set_fg(Some(SOLARIZED_BASE1))
                .set_bg(Some(SOLARIZED_BASE02))
                .set_bold(true),
        )?;
        write!(inner.stdout, "{header}")?;
        inner.stdout.reset()?;
        writeln!(inner.stdout)?;
        inner.stdout.flush()
    }

    pub fn warn(&self, message: &str) -> io::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .stdout
            .set_color(ColorSpec::new().set_fg(Some(SOLARIZED_YELLOW)))?;
        writeln!(inner.stdout, "[WARNING] {message}")?;
        inner.stdout.reset()?;
        inner.stdout.flush()
    }

    pub fn error(&self, message: &str) -> io::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .stdout
            .set_color(ColorSpec::new().set_fg(Some(SOLARIZED_RED)).set_bold(true))?;
        writeln!(inner.stdout, "[ERROR] {message}")?;
        inner.stdout.reset()?;
        inner.stdout.flush()
    }

    pub fn success(&self, message: &str) -> io::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .stdout
            .set_color(ColorSpec::new().set_fg(Some(SOLARIZED_GREEN)))?;
        writeln!(inner.stdout, "[OK] {message}")?;
        inner.stdout.reset()?;
        inner.stdout.flush()
    }

    pub fn debug(&self, message: &str) -> io::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .stdout
            .set_color(ColorSpec::new().set_fg(Some(SOLARIZED_MAGENTA)))?;
        writeln!(inner.stdout, "[DEBUG] {message}")?;
        inner.stdout.reset()?;
        inner.stdout.flush()
    }

    pub fn trace(&self, message: &str, level: Level) -> io::Result<()> {
        let mut inner = self.inner.lock().unwrap();

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

        inner.stdout.set_color(&color_spec)?;
        writeln!(inner.stdout, "trace: {message}")?;
        inner.stdout.reset()?;
        inner.stdout.flush()
    }
}

impl Default for Output {
    fn default() -> Self {
        Self::new()
    }
}

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
