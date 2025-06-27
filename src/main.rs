mod common;
mod connect;
mod listtools;
mod output;
mod ping;
mod proxy;
mod target;
mod testserver;
mod utils;

use clap::{Args, Parser, Subcommand};
use mcptool::VERSION;
use target::Target;

#[derive(Args)]
struct TargetArgs {
    /// The MCP server target (e.g., "localhost:3000", "tcp://host:port", "http://host:port", "https://host:port", "cmd://./server")
    target: String,
}

#[derive(Args)]
struct ProxyArgs {
    /// The MCP server target to proxy to (e.g., "localhost:3000", "tcp://host:port", "http://host:port", "https://host:port", "cmd://./server")
    target: String,

    /// File path to log all proxy traffic
    #[arg(long)]
    log_file: std::path::PathBuf,
}

#[derive(Parser)]
#[command(
    name = "mcptool",
    about = "A versatile command-line utility for connecting to, testing, and probing MCP servers",
    version = VERSION,
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
        #[command(flatten)]
        target_args: TargetArgs,
    },

    /// List all MCP tools from a server
    Listtools {
        #[command(flatten)]
        target_args: TargetArgs,
    },

    /// Connect to an MCP server and start an interactive REPL
    Connect {
        #[command(flatten)]
        target_args: TargetArgs,
    },

    /// Transparently proxy and print traffic forwarded to the target
    Proxy {
        #[command(flatten)]
        proxy_args: ProxyArgs,
    },

    /// Run a test MCP server with verbose logging
    Testserver {
        /// Use stdio transport instead of HTTP
        #[arg(long)]
        stdio: bool,

        /// Port to listen on (for HTTP transport)
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Enable tracing with optional level (error, warn, info, debug, trace)
        #[arg(long, value_name = "LEVEL")]
        trace: Option<Option<String>>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Version => {
            println!("mcptool version {VERSION}");
            println!(
                "MCP protocol version: {}",
                tenx_mcp::schema::LATEST_PROTOCOL_VERSION
            );
        }

        Commands::Ping { target_args } => {
            let target = Target::parse(&target_args.target)?;
            ping::ping_command(target).await?;
        }

        Commands::Listtools { target_args } => {
            let target = Target::parse(&target_args.target)?;
            listtools::listtools_command(target).await?;
        }

        Commands::Connect { target_args } => {
            let target = Target::parse(&target_args.target)?;
            connect::connect_command(target).await?;
        }

        Commands::Proxy { proxy_args } => {
            let target = Target::parse(&proxy_args.target)?;
            proxy::proxy_command(target, proxy_args.log_file).await?;
        }

        Commands::Testserver { stdio, port, trace } => {
            testserver::run_test_server(stdio, port, trace).await?;
        }
    }

    Ok(())
}
