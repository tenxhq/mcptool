use clap::{Args, Parser, Subcommand};
use libmcptool::{auth, connect, ctx, mcp, proxy, target::Target, testserver};

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
    version = ctx::VERSION,
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Display the mcptool build version & linked MCP revision
    Version,

    /// MCP invocation commands (ping, listtools, etc.)
    Mcp {
        #[command(subcommand)]
        command: mcp::McpCommands,
    },

    /// Connect to an MCP server and start an interactive REPL
    Connect {
        /// The MCP server target (e.g., "localhost:3000", "tcp://host:port", "http://host:port")
        /// Optional when using --auth, will use the stored server URL
        target: Option<String>,

        /// Enable logging with optional level (error, warn, info, debug, trace)
        #[arg(long, value_name = "LEVEL")]
        logs: Option<Option<String>>,

        /// Use a stored authentication entry
        #[arg(long)]
        auth: Option<String>,
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

        /// Enable logging with optional level (error, warn, info, debug, trace)
        #[arg(long, value_name = "LEVEL")]
        logs: Option<Option<String>>,
    },

    /// Manage OAuth authentication entries
    Auth {
        #[command(subcommand)]
        command: auth::AuthCommands,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Calculate the configuration directory
    let config_path = dirs::config_dir()
        .ok_or("Failed to get config directory")?
        .join("mcptool");

    // Determine if any command needs logging
    let logs = match &cli.command {
        Commands::Connect { logs, .. } => logs.clone(),
        Commands::Testserver { logs, .. } => logs.clone(),
        _ => None,
    };

    // Create the MCPTool instance
    let ctx = ctx::Ctx::new(config_path, logs)?;

    match cli.command {
        Commands::Version => {
            println!("mcptool version {}", ctx::VERSION);
            println!(
                "MCP protocol version: {}",
                tenx_mcp::schema::LATEST_PROTOCOL_VERSION
            );
        }

        Commands::Mcp { command } => {
            mcp::handle_mcp_command(&ctx, command).await?;
        }

        Commands::Connect { target, auth, .. } => {
            connect::connect_command(&ctx, target, auth).await?;
        }

        Commands::Proxy { proxy_args } => {
            let target = Target::parse(&proxy_args.target)?;
            proxy::proxy_command(target, proxy_args.log_file).await?;
        }

        Commands::Testserver { stdio, port, .. } => {
            testserver::run_test_server(&ctx, stdio, port).await?;
        }

        Commands::Auth { command } => {
            auth::handle_auth_command(&ctx, command).await?;
        }
    }

    Ok(())
}
