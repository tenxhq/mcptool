mod listtools;
mod ping;

use crate::core::MCPTool;
use crate::output::Output;
use crate::target::Target;
use clap::{Args, Subcommand};
use std::sync::Arc;
use tenx_mcp::auth::{OAuth2Client, OAuth2Config};

pub use listtools::listtools_command;
pub use ping::ping_command;

#[derive(Args)]
pub struct McpArgs {
    /// Use a stored authentication entry
    #[arg(long)]
    auth: Option<String>,
}

#[derive(Subcommand)]
pub enum McpCommands {
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

pub async fn handle_mcp_command(
    command: McpCommands,
    mcptool: &MCPTool,
    output: Output,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        McpCommands::Ping { target, mcp_args } => {
            let target = resolve_target(target, &mcp_args.auth, mcptool)?;
            ping_command(target, mcp_args.auth, mcptool, output).await
        }
        McpCommands::Listtools { target, mcp_args } => {
            let target = resolve_target(target, &mcp_args.auth, mcptool)?;
            listtools_command(target, mcp_args.auth, mcptool, output).await
        }
    }
}

fn resolve_target(
    target: Option<String>,
    auth_name: &Option<String>,
    mcptool: &MCPTool,
) -> Result<Target, Box<dyn std::error::Error>> {
    match (target, auth_name) {
        (Some(t), _) => {
            // Target provided, parse it
            Ok(Target::parse(&t)?)
        }
        (None, Some(auth)) => {
            // No target but auth provided, get URL from auth
            let storage = mcptool.storage()?;
            let auth_entry = storage.get_auth(auth)?;

            println!(
                "Using server URL from auth '{}': {}",
                auth, auth_entry.server_url
            );

            Ok(Target::parse(&auth_entry.server_url)?)
        }
        (None, None) => {
            Err("No target specified. Either provide a target URL or use --auth".into())
        }
    }
}

pub async fn connect_with_auth(
    target: &Target,
    auth_name: &str,
    mcptool: &MCPTool,
    output: &Output,
) -> Result<(tenx_mcp::Client<()>, tenx_mcp::schema::InitializeResult), Box<dyn std::error::Error>>
{
    // Only HTTP/HTTPS targets support OAuth
    match target {
        Target::Http { .. } | Target::Https { .. } => {}
        _ => return Err("OAuth authentication is only supported for HTTP/HTTPS targets".into()),
    }

    // Load auth credentials
    let storage = mcptool.storage()?;
    let auth = storage.get_auth(auth_name)?;

    output.text(format!("Using authentication: {auth_name}"))?;

    // Check if token is expired
    if let Some(expires_at) = auth.expires_at {
        if expires_at <= std::time::SystemTime::now() {
            output.warn("Access token has expired. Token refresh not yet implemented.")?;
            return Err(
                "Access token has expired. Please re-authenticate with 'mcptool auth add'".into(),
            );
        }
    }

    // Create OAuth config
    let oauth_config = OAuth2Config {
        client_id: auth.client_id,
        client_secret: auth.client_secret,
        auth_url: auth.auth_url,
        token_url: auth.token_url,
        redirect_url: auth
            .redirect_url
            .unwrap_or_else(|| "http://localhost:0".to_string()),
        resource: "".to_string(), // Empty resource, could be stored in auth if needed
        scopes: auth.scopes,
    };

    // Create OAuth client
    let oauth_client = OAuth2Client::new(oauth_config)?;

    // Set the stored tokens if available
    if let Some(access_token) = auth.access_token {
        let token = tenx_mcp::auth::OAuth2Token {
            access_token,
            refresh_token: auth.refresh_token,
            expires_at: auth.expires_at.map(|system_time| {
                // Convert SystemTime to Instant
                match system_time.duration_since(std::time::SystemTime::now()) {
                    Ok(duration) => std::time::Instant::now() + duration,
                    Err(_) => std::time::Instant::now(), // Token is already expired
                }
            }),
        };
        oauth_client.set_token(token).await;
    }

    let oauth_client = Arc::new(oauth_client);

    // Create MCP client
    let mut client = tenx_mcp::Client::new("mcptool", crate::VERSION)
        .with_capabilities(tenx_mcp::schema::ClientCapabilities::default());

    // Connect with OAuth
    let init_result = match target {
        Target::Http { host, port } => {
            let url = format!("http://{host}:{port}");
            client
                .connect_http_with_oauth(&url, oauth_client)
                .await
                .map_err(|e| format!("Failed to connect to HTTP endpoint {url} with OAuth: {e}"))?
        }
        Target::Https { host, port } => {
            let url = format!("https://{host}:{port}");
            client
                .connect_http_with_oauth(&url, oauth_client)
                .await
                .map_err(|e| format!("Failed to connect to HTTPS endpoint {url} with OAuth: {e}"))?
        }
        _ => unreachable!(), // We checked this above
    };

    Ok((client, init_result))
}
