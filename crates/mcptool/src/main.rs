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

#[derive(Args)]
struct McpArgs {
    /// Use a stored authentication entry
    #[arg(long)]
    auth: Option<String>,
}

#[derive(Subcommand)]
enum McpCommands {
    /// Send a ping request to an MCP server
    Ping {
        /// The MCP server target (e.g., "localhost:3000", "tcp://host:port", "http://host:port")
        /// Optional when using --auth, will use the stored server URL
        target: Option<String>,

        #[command(flatten)]
        mcp_args: McpArgs,
    },

    /// List all MCP tools from a server
    Listtools {
        /// The MCP server target (e.g., "localhost:3000", "tcp://host:port", "http://host:port")
        /// Optional when using --auth, will use the stored server URL
        target: Option<String>,

        #[command(flatten)]
        mcp_args: McpArgs,
    },
}

#[derive(Subcommand)]
enum AuthCommands {
    /// Add a new OAuth authentication entry
    Add {
        /// Name for the authentication entry
        name: String,

        /// Server URL (e.g., https://api.example.com)
        #[arg(long)]
        server_url: Option<String>,

        /// OAuth authorization URL
        #[arg(long)]
        auth_url: Option<String>,

        /// OAuth token URL
        #[arg(long)]
        token_url: Option<String>,

        /// OAuth client ID
        #[arg(long)]
        client_id: Option<String>,

        /// OAuth client secret
        #[arg(long)]
        client_secret: Option<String>,

        /// OAuth redirect URL (if not provided, will use local server)
        #[arg(long)]
        redirect_url: Option<String>,

        /// Resource/Audience parameter for OAuth
        #[arg(long)]
        resource: Option<String>,

        /// OAuth scopes (comma-separated)
        #[arg(long)]
        scopes: Option<String>,

        /// Show the redirect URL that will be used without starting OAuth flow
        #[arg(long)]
        show_redirect_url: bool,
    },

    /// List all stored authentication entries
    #[command(alias = "ls")]
    List,

    /// Remove an authentication entry
    #[command(alias = "rm")]
    Remove {
        /// Name of the authentication entry to remove
        name: String,
    },

    /// Renew the access token for an authentication entry using the refresh token
    Renew {
        /// Name of the authentication entry to renew
        name: String,
    },
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
        command: McpCommands,
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
        command: AuthCommands,
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

        Commands::Mcp { command } => match command {
            McpCommands::Ping { target, mcp_args } => {
                mcp::handle_ping_command(&ctx, target, mcp_args.auth).await?;
            }
            McpCommands::Listtools { target, mcp_args } => {
                mcp::handle_listtools_command(&ctx, target, mcp_args.auth).await?;
            }
        },

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

        Commands::Auth { command } => match command {
            AuthCommands::Add {
                name,
                server_url,
                auth_url,
                token_url,
                client_id,
                client_secret,
                redirect_url,
                resource,
                scopes,
                show_redirect_url,
            } => {
                let args = auth::AddCommandArgs {
                    name,
                    server_url,
                    auth_url,
                    token_url,
                    client_id,
                    client_secret,
                    redirect_url,
                    resource,
                    scopes,
                    show_redirect_url,
                };
                auth::add_command(&ctx, args).await?;
            }
            AuthCommands::List => auth::list_command(&ctx).await?,
            AuthCommands::Remove { name } => auth::remove_command(&ctx, name).await?,
            AuthCommands::Renew { name } => auth::renew_command(&ctx, name).await?,
        },
    }

    Ok(())
}
