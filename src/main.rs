mod target;

use clap::{Parser, Subcommand};
use target::Target;
use tenx_mcp::{
    client::MCPClient,
    schema::{ClientCapabilities, Implementation},
    transport::{StdioTransport, TcpTransport, Transport},
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
    let transport: Box<dyn Transport> = match target {
        Target::Tcp { host, port } => {
            let addr = format!("{}:{}", host, port);
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

    let mut client = MCPClient::new();
    client.connect(transport).await?;

    // Initialize the connection
    let client_info = Implementation {
        name: "mcptool".to_string(),
        version: VERSION.to_string(),
    };

    let capabilities = ClientCapabilities::default();

    client.initialize(client_info, capabilities).await?;

    // Send the actual ping request
    client.ping().await?;

    Ok(())
}
