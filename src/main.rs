mod target;

use clap::{Parser, Subcommand};
use target::Target;
use tenx_mcp::{
    client::MCPClient,
    schema::{ClientCapabilities, Implementation},
    transport::{StdioTransport, TcpTransport, Transport},
};
use tokio::time::{Duration, Instant, timeout};

const VERSION: &str = env!("CARGO_PKG_VERSION");

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

    /// Measure round-trip latency to an MCP server
    Ping {
        /// The MCP server target (e.g., "api.example.com", "tcp://host:port", "cmd://./server")
        target: String,

        /// Number of ping attempts
        #[arg(short = 'c', long, default_value = "4")]
        count: u32,

        /// Timeout for each ping in milliseconds
        #[arg(short = 't', long, default_value = "5000")]
        timeout_ms: u64,
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

        Commands::Ping {
            target,
            count,
            timeout_ms,
        } => {
            let target = Target::parse(&target)?;
            ping_command(target, count, timeout_ms).await?;
        }
    }

    Ok(())
}

async fn ping_command(
    target: Target,
    count: u32,
    timeout_ms: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Pinging {} with {} attempts...", target, count);

    let mut successful_pings = 0;
    let mut total_time = Duration::from_secs(0);
    let mut min_time = Duration::from_millis(u64::MAX);
    let mut max_time = Duration::from_secs(0);

    for i in 0..count {
        match ping_once(&target, Duration::from_millis(timeout_ms)).await {
            Ok(duration) => {
                successful_pings += 1;
                total_time += duration;
                min_time = min_time.min(duration);
                max_time = max_time.max(duration);
                println!("Ping #{}: {} ms", i + 1, duration.as_millis());
            }
            Err(e) => {
                println!("Ping #{}: Failed - {}", i + 1, e);
            }
        }

        // Small delay between pings
        if i < count - 1 {
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    println!("\n--- {} ping statistics ---", target);
    println!(
        "{} pings transmitted, {} successful, {}% packet loss",
        count,
        successful_pings,
        ((count - successful_pings) * 100) / count
    );

    if successful_pings > 0 {
        let avg_time = total_time / successful_pings;
        println!(
            "round-trip min/avg/max = {:.1}/{:.1}/{:.1} ms",
            min_time.as_secs_f64() * 1000.0,
            avg_time.as_secs_f64() * 1000.0,
            max_time.as_secs_f64() * 1000.0
        );
    }

    Ok(())
}

async fn ping_once(
    target: &Target,
    timeout_duration: Duration,
) -> Result<Duration, Box<dyn std::error::Error>> {
    let start = Instant::now();

    let transport: Box<dyn Transport> = match target {
        Target::Tcp { host, port } => {
            let addr = if let Some(p) = port {
                format!("{}:{}", host, p)
            } else {
                // Use default MCP port if not specified
                format!("{}:9000", host)
            };
            Box::new(TcpTransport::new(addr))
        }
        Target::Stdio { command, args } => {
            // For stdio, we need to spawn the process
            let mut cmd = tokio::process::Command::new(command);
            cmd.args(args);
            cmd.stdin(std::process::Stdio::piped());
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::null());

            let mut _child = cmd.spawn()?;

            // For now, we'll use the standard stdio transport
            // In a real implementation, we would need to properly connect
            // the child process stdin/stdout to the transport
            Box::new(StdioTransport::new())
        }
    };

    // Perform the ping with timeout
    let result = timeout(timeout_duration, async {
        let mut client = MCPClient::new();
        client.connect(transport).await?;

        // Initialize the connection
        let client_info = Implementation {
            name: "mcptool".to_string(),
            version: VERSION.to_string(),
        };

        let capabilities = ClientCapabilities::default();

        client.initialize(client_info, capabilities).await?;

        Ok::<_, Box<dyn std::error::Error>>(())
    })
    .await;

    match result {
        Ok(Ok(())) => Ok(start.elapsed()),
        Ok(Err(e)) => Err(format!("Connection failed: {}", e).into()),
        Err(_) => Err("Connection timed out".into()),
    }
}
