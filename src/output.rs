use std::io::{self, Write};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

pub struct Output {
    stdout: StandardStream,
}

impl Output {
    pub fn new() -> Self {
        Self {
            stdout: StandardStream::stdout(ColorChoice::Auto),
        }
    }

    pub fn text(&mut self, message: &str) -> io::Result<()> {
        self.stdout.reset()?;
        writeln!(self.stdout, "{message}")?;
        self.stdout.flush()
    }

    pub fn heading(&mut self, message: &str) -> io::Result<()> {
        self.stdout
            .set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))?;
        writeln!(self.stdout, "# {message}")?;
        self.stdout.reset()?;
        self.stdout.flush()
    }

    pub fn warn(&mut self, message: &str) -> io::Result<()> {
        self.stdout
            .set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
        writeln!(self.stdout, "⚠ {message}")?;
        self.stdout.reset()?;
        self.stdout.flush()
    }

    pub fn error(&mut self, message: &str) -> io::Result<()> {
        self.stdout
            .set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
        writeln!(self.stdout, "✗ {message}")?;
        self.stdout.reset()?;
        self.stdout.flush()
    }

    pub fn success(&mut self, message: &str) -> io::Result<()> {
        self.stdout
            .set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
        writeln!(self.stdout, "✓ {message}")?;
        self.stdout.reset()?;
        self.stdout.flush()
    }
}

impl Default for Output {
    fn default() -> Self {
        Self::new()
    }
}
