mod target;

use clap::{Parser, Subcommand};
use std::pin::Pin;
use std::task::{Context, Poll};
use target::Target;
use tenx_mcp::{
    client::Client,
    schema::{ClientCapabilities, Implementation},
    transport::{StreamTransport, Transport},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// A duplex stream wrapper around child process stdin/stdout
pub struct ChildProcessDuplex {
    stdin: tokio::process::ChildStdin,
    stdout: tokio::process::ChildStdout,
}

impl ChildProcessDuplex {
    pub fn new(stdin: tokio::process::ChildStdin, stdout: tokio::process::ChildStdout) -> Self {
        Self { stdin, stdout }
    }
}

impl AsyncRead for ChildProcessDuplex {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stdout).poll_read(cx, buf)
    }
}

impl AsyncWrite for ChildProcessDuplex {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut self.stdin).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.stdin).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.stdin).poll_shutdown(cx)
    }
}

#[derive(Parser)]
#[command(
    name = "mcptool",
    about = "A versatile command-line utility for connecting to, testing, and probing MCP servers",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Display the mcptool build version & linked MCP revision
    Version,

    /// Send a ping request to an MCP server
    Ping {
        /// The MCP server target (e.g., "api.example.com", "tcp://host:port", "cmd://./server")
        target: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Version => {
            println!("mcptool version {}", VERSION);
            println!(
                "MCP protocol version: {}",
                tenx_mcp::schema::LATEST_PROTOCOL_VERSION
            );
        }

        Commands::Ping { target } => {
            let target = Target::parse(&target)?;
            ping_command(target).await?;
        }
    }

    Ok(())
}

async fn ping_command(target: Target) -> Result<(), Box<dyn std::error::Error>> {
    println!("Pinging {}...", target);

    match ping_once(&target).await {
        Ok(()) => {
            println!("Ping successful");
        }
        Err(e) => {
            println!("Ping failed: {}", e);
        }
    }

    Ok(())
}

async fn ping_once(target: &Target) -> Result<(), Box<dyn std::error::Error>> {
    use std::time::Instant;

    let start_time = Instant::now();

    let transport: Box<dyn Transport> = match target {
        Target::Tcp { host, port } => {
            let addr = format!("{}:{}", host, port);
            let stream = tokio::net::TcpStream::connect(&addr)
                .await
                .map_err(|e| format!("Failed to connect to TCP address {}: {}", addr, e))?;
            Box::new(StreamTransport::new(stream))
        }
        Target::Stdio { command, args } => {
            println!(
                "Connecting to MCP server via command: {} {}",
                command,
                args.join(" ")
            );

            let mut cmd = tokio::process::Command::new(command);
            cmd.args(args);
            cmd.stdin(std::process::Stdio::piped());
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());

            let mut child = cmd
                .spawn()
                .map_err(|e| format!("Failed to spawn MCP server process: {}", e))?;

            // Extract stdin and stdout from the child process
            let stdin = child
                .stdin
                .take()
                .ok_or("Failed to get stdin from child process")?;
            let stdout = child
                .stdout
                .take()
                .ok_or("Failed to get stdout from child process")?;

            // Create a duplex stream from the child process streams
            let duplex = ChildProcessDuplex::new(stdin, stdout);

            // Use StreamTransport to wrap the duplex stream
            Box::new(StreamTransport::new(duplex))
        }
    };

    let mut client = Client::new();

    // Track connection time
    let connect_start = Instant::now();
    client
        .connect(transport)
        .await
        .map_err(|e| format!("Failed to connect to MCP server: {}", e))?;
    let connect_duration = connect_start.elapsed();

    println!(
        "Connected in {:.2}ms",
        connect_duration.as_secs_f64() * 1000.0
    );

    // Initialize the connection
    let client_info = Implementation {
        name: "mcptool".to_string(),
        version: VERSION.to_string(),
    };

    let capabilities = ClientCapabilities::default();

    let init_start = Instant::now();
    let init_result = client
        .initialize(client_info, capabilities)
        .await
        .map_err(|e| format!("Failed to initialize MCP client: {}", e))?;
    let init_duration = init_start.elapsed();

    println!(
        "Initialized in {:.2}ms",
        init_duration.as_secs_f64() * 1000.0
    );
    println!(
        "Server info: {} v{}",
        init_result.server_info.name, init_result.server_info.version
    );

    // Send the actual ping request with timing
    let ping_start = Instant::now();
    client
        .ping()
        .await
        .map_err(|e| format!("Ping request failed: {}", e))?;
    let ping_duration = ping_start.elapsed();

    let total_duration = start_time.elapsed();

    println!(
        "Ping successful in {:.2}ms",
        ping_duration.as_secs_f64() * 1000.0
    );
    println!("Total time: {:.2}ms", total_duration.as_secs_f64() * 1000.0);

    Ok(())
}
