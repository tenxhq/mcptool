use clap::{Args, Parser, Subcommand};
use tenx_mcp::{Client, ClientConn, schema::InitializeResult};

use crate::{Result, client, ctx::Ctx, mcp, target::Target};

#[derive(Args)]
pub struct McpArgs {
    // No longer needed - auth is now handled via auth:// target syntax
}

// Base commands without target - used by both CLI and REPL
#[derive(Subcommand)]
#[command(no_binary_name = true)]
pub enum McpCommand {
    /// Send a ping request to an MCP server
    Ping,

    /// List all MCP tools from a server
    Listtools,

    /// Initialize connection and display server information
    Init,

    /// List all MCP resources from a server
    Listresources,

    /// List all MCP prompts from a server
    Listprompts,

    /// List all MCP resource templates from a server
    Listresourcetemplates,

    /// Set the logging level on the MCP server
    SetLevel {
        /// The logging level to set (debug, info, notice, warning, error, critical, alert, emergency)
        level: String,
    },

    /// Call an MCP tool with various input modes
    Calltool {
        /// Name of the tool to call
        tool_name: String,

        /// Arguments in key=value format (can be specified multiple times)
        #[arg(long = "arg", short = 'a')]
        args: Vec<String>,

        /// Interactive mode: prompt for each tool parameter
        #[arg(long, short)]
        interactive: bool,

        /// JSON mode: read arguments from stdin as JSON
        #[arg(long, short)]
        json: bool,
    },

    /// Read a resource by URI
    ReadResource {
        /// URI of the resource to read
        uri: String,
    },

    /// Get a prompt by name with optional arguments
    GetPrompt {
        /// Name of the prompt to get
        name: String,

        /// Arguments in key=value format (can be specified multiple times)
        #[arg(long = "arg", short = 'a')]
        args: Vec<String>,
    },

    /// Subscribe to resource update notifications
    SubscribeResource {
        /// URI of the resource to subscribe to
        uri: String,
    },

    /// Unsubscribe from resource update notifications
    UnsubscribeResource {
        /// URI of the resource to unsubscribe from
        uri: String,
    },

    /// Get completion suggestions for prompt or resource arguments
    Complete {
        /// Reference to complete (e.g., "resource://uri" or "prompt://name")
        reference: String,

        /// Name of the argument to complete
        argument: String,
    },
}

// For CLI use - target is required at this level
#[derive(Parser)]
pub struct CliMcpCommand {
    /// The MCP server target (e.g., "localhost:3000", "tcp://host:port", "http://host:port", "auth://name")
    pub target: String,

    #[command(subcommand)]
    pub command: McpCommand,
}

// For REPL use - no target needed, just the command
#[derive(Parser)]
#[command(no_binary_name = true)]
pub struct ReplCommandWrapper {
    #[command(subcommand)]
    pub command: McpCommand,
}

// For REPL use - reuses existing client connection
pub async fn execute_mcp_command_with_client<C: ClientConn + 'static>(
    command: McpCommand,
    client: &mut Client<C>,
    init_result: &InitializeResult,
    ctx: &Ctx,
) -> Result<()> {
    match command {
        McpCommand::Ping => {
            mcp::ping(client, &ctx.output).await?;
        }
        McpCommand::Listtools => {
            mcp::listtools(client, &ctx.output).await?;
        }
        McpCommand::Init => {
            mcp::init(init_result, &ctx.output)?;
        }
        McpCommand::Listresources => {
            mcp::listresources(client, &ctx.output).await?;
        }
        McpCommand::Listprompts => {
            mcp::listprompts(client, &ctx.output).await?;
        }
        McpCommand::Listresourcetemplates => {
            mcp::listresourcetemplates(client, &ctx.output).await?;
        }
        McpCommand::SetLevel { level } => {
            mcp::set_level(client, &ctx.output, &level).await?;
        }
        McpCommand::Calltool {
            tool_name,
            args,
            interactive,
            json,
        } => {
            mcp::calltool(client, &ctx.output, &tool_name, args, interactive, json).await?;
        }
        McpCommand::ReadResource { uri } => {
            mcp::read_resource(client, &ctx.output, &uri).await?;
        }
        McpCommand::GetPrompt { name, args } => {
            mcp::get_prompt(client, &ctx.output, &name, args).await?;
        }
        McpCommand::SubscribeResource { uri } => {
            mcp::subscribe_resource(client, &ctx.output, &uri).await?;
        }
        McpCommand::UnsubscribeResource { uri } => {
            mcp::unsubscribe_resource(client, &ctx.output, &uri).await?;
        }
        McpCommand::Complete {
            reference,
            argument,
        } => {
            mcp::complete(client, &ctx.output, &reference, &argument).await?;
        }
    }
    Ok(())
}

// For CLI use - creates new client connection for single command
pub async fn execute_mcp_command(command: McpCommand, target: &str, ctx: &Ctx) -> Result<()> {
    let target = Target::parse(target)?;
    let (mut client, init_result) = client::get_client(ctx, &target).await?;
    execute_mcp_command_with_client(command, &mut client, &init_result, ctx).await
}
