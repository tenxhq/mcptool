mod target;

use clap::{Parser, Subcommand};
use target::Target;
use tenx_mcp::{
    Client,
    schema::{ClientCapabilities, Implementation},
};

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
    let mut client = Client::new();

    // Track connection and initialization time
    let connect_start = Instant::now();

    let init_result = match target {
        Target::Tcp { host, port } => {
            let addr = format!("{}:{}", host, port);
            client
                .connect_tcp(&addr, "mcptool", VERSION)
                .await
                .map_err(|e| format!("Failed to connect to TCP address {}: {}", addr, e))?
        }
        Target::Stdio { command, args } => {
            println!(
                "Connecting to MCP server via command: {} {}",
                command,
                args.join(" ")
            );

            let mut cmd = tokio::process::Command::new(command);
            cmd.args(args);

            let _child = client
                .connect_process(cmd)
                .await
                .map_err(|e| format!("Failed to spawn MCP server process: {}", e))?;

            // Initialize the connection
            let client_info = Implementation {
                name: "mcptool".to_string(),
                version: VERSION.to_string(),
            };

            let capabilities = ClientCapabilities::default();

            client
                .initialize(client_info, capabilities)
                .await
                .map_err(|e| format!("Failed to initialize MCP client: {}", e))?
        }
    };

    let connect_duration = connect_start.elapsed();

    println!(
        "Connected and initialized in {:.2}ms",
        connect_duration.as_secs_f64() * 1000.0
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
