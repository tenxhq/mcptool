use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

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
        inner
            .stdout
            .set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))?;
        writeln!(inner.stdout, "# {message}")?;
        inner.stdout.reset()?;
        inner.stdout.flush()
    }

    pub fn warn(&self, message: &str) -> io::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .stdout
            .set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
        writeln!(inner.stdout, "⚠ {message}")?;
        inner.stdout.reset()?;
        inner.stdout.flush()
    }

    pub fn error(&self, message: &str) -> io::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .stdout
            .set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
        writeln!(inner.stdout, "✗ {message}")?;
        inner.stdout.reset()?;
        inner.stdout.flush()
    }

    pub fn success(&self, message: &str) -> io::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .stdout
            .set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
        writeln!(inner.stdout, "✓ {message}")?;
        inner.stdout.reset()?;
        inner.stdout.flush()
    }

    pub fn debug(&self, message: &str) -> io::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .stdout
            .set_color(ColorSpec::new().set_fg(Some(Color::Magenta)))?;
        writeln!(inner.stdout, "• {message}")?;
        inner.stdout.reset()?;
        inner.stdout.flush()
    }

    pub fn trace(&self, message: &str, level: Level) -> io::Result<()> {
        let mut inner = self.inner.lock().unwrap();

        let mut color_spec = ColorSpec::new();
        match level {
            Level::ERROR => {
                color_spec.set_fg(Some(Color::Red)).set_bold(true);
            }
            Level::WARN => {
                color_spec.set_fg(Some(Color::Yellow));
            }
            Level::INFO => {
                color_spec.set_dimmed(true); // Grey/dimmed white
            }
            Level::DEBUG => {
                color_spec.set_fg(Some(Color::Magenta));
            }
            Level::TRACE => {
                color_spec.set_fg(Some(Color::Blue)).set_dimmed(true);
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
            self.message = format!("{:?}", value);
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }
}
