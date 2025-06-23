mod proxy;
mod target;
mod utils;

use clap::{Args, Parser, Subcommand};
use rustyline::DefaultEditor;
use target::Target;
use tenx_mcp::{
    Client, ServerAPI,
    schema::{ClientCapabilities, InitializeResult},
};
use utils::TimedFuture;

pub const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "-",
    env!("VERGEN_GIT_SHA"),
    " (",
    env!("VERGEN_BUILD_DATE"),
    ")"
);

#[derive(Args)]
struct TargetArgs {
    /// The MCP server target (e.g., "api.example.com", "tcp://host:port", "cmd://./server")
    target: String,
}

#[derive(Args)]
struct ProxyArgs {
    /// The MCP server target to proxy to (e.g., "api.example.com", "tcp://host:port", "cmd://./server")
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

        Commands::Ping { target_args } => {
            let target = Target::parse(&target_args.target)?;
            ping_command(target).await?;
        }

        Commands::Listtools { target_args } => {
            let target = Target::parse(&target_args.target)?;
            listtools_command(target).await?;
        }

        Commands::Connect { target_args } => {
            let target = Target::parse(&target_args.target)?;
            connect_command(target).await?;
        }

        Commands::Proxy { proxy_args } => {
            let target = Target::parse(&proxy_args.target)?;
            proxy::proxy_command(target, proxy_args.log_file).await?;
        }
    }

    Ok(())
}

async fn ping_command(target: Target) -> Result<(), Box<dyn std::error::Error>> {
    println!("Pinging {}...", target);

    ping_once(&target).await?;

    Ok(())
}

async fn connect_to_server(
    target: &Target,
) -> Result<(Client<()>, InitializeResult), Box<dyn std::error::Error>> {
    let mut client = Client::new("mcptool", VERSION)
        .with_capabilities(ClientCapabilities::default());

    let init_result = match target {
        Target::Tcp { host, port } => {
            let addr = format!("{}:{}", host, port);
            client
                .connect_tcp(&addr)
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

            // The new API handles initialization automatically
            client
                .init()
                .await
                .map_err(|e| format!("Failed to initialize MCP client: {}", e))?
        }
    };

    Ok((client, init_result))
}

async fn execute_ping(client: &mut Client<()>) -> Result<(), Box<dyn std::error::Error>> {
    client.ping().timed("Pinged").await?;
    Ok(())
}

async fn ping_once(target: &Target) -> Result<(), Box<dyn std::error::Error>> {
    let (mut client, init_result) = connect_to_server(target)
        .timed("Connected and initialized")
        .await?;

    println!(
        "Server info: {} v{}",
        init_result.server_info.name, init_result.server_info.version
    );

    execute_ping(&mut client).await?;

    Ok(())
}

fn display_tools(
    tools_result: &tenx_mcp::schema::ListToolsResult,
) -> Result<(), Box<dyn std::error::Error>> {
    if tools_result.tools.is_empty() {
        println!("No tools available from this server.");
    } else {
        println!("Available tools ({}):\n", tools_result.tools.len());
        for tool in &tools_result.tools {
            println!("  - {}", tool.name);

            println!("\n    Description:\n");
            match &tool.description {
                Some(description) => {
                    for line in description.lines() {
                        println!("      {}", line);
                    }
                }
                None => println!("      No description available"),
            }

            println!("\n    Annotations:\n");
            match &tool.annotations {
                Some(annotations) => {
                    println!("      {:?}", annotations.title);
                }
                None => println!("      No annotations available"),
            }

            println!("\n    Input arguments:\n");

            // TODO Show required inputs first?
            match &tool.input_schema.properties {
                Some(properties) => {
                    for (name, schema) in properties {
                        let rendered_schema = serde_json::to_string_pretty(schema)
                            .map_err(|e| format!("Failed to serialize schema: {}", e))?;
                        let is_required = &tool
                            .clone()
                            .input_schema
                            .required
                            .is_some_and(|list| list.contains(name));
                        println!("      {} - (required: {})\n", name, is_required);

                        for line in rendered_schema.lines() {
                            println!("        {}", line);
                        }
                        println!();
                    }
                }
                None => println!("      No input schema available"),
            }

            println!(); // Extra blank line between tools
        }
    }
    Ok(())
}

async fn execute_listtools(client: &mut Client<()>) -> Result<(), Box<dyn std::error::Error>> {
    let tools_result = client.list_tools(None).timed("Tools retrieved").await?;
    display_tools(&tools_result)?;
    Ok(())
}

async fn listtools_command(target: Target) -> Result<(), Box<dyn std::error::Error>> {
    println!("Listing tools from {}...", target);

    let (mut client, init_result) = connect_to_server(&target).await?;

    println!(
        "Connected to: {} v{}\n",
        init_result.server_info.name, init_result.server_info.version
    );

    execute_listtools(&mut client).await?;

    Ok(())
}

async fn connect_command(target: Target) -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to {}...", target);

    let (mut client, init_result) = connect_to_server(&target).await?;

    println!(
        "Connected to: {} v{}",
        init_result.server_info.name, init_result.server_info.version
    );
    println!("Type 'help' for available commands, 'quit' to exit\n");

    let mut rl = DefaultEditor::new()?;

    loop {
        let readline = rl.readline("mcp> ");
        match readline {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                rl.add_history_entry(line)?;

                match line {
                    "quit" | "exit" => {
                        println!("Goodbye!");
                        break;
                    }
                    "help" => {
                        println!("Available commands:");
                        println!("  ping      - Send a ping request to the server");
                        println!("  listtools - List all available tools from the server");
                        println!("  help      - Show this help message");
                        println!("  quit/exit - Exit the REPL");
                    }
                    "ping" => match execute_ping(&mut client).await {
                        Ok(_) => println!("Ping successful!"),
                        Err(e) => println!("Ping failed: {}", e),
                    },
                    "listtools" => match execute_listtools(&mut client).await {
                        Ok(_) => {}
                        Err(e) => println!("Failed to list tools: {}", e),
                    },
                    _ => {
                        println!(
                            "Unknown command: {}. Type 'help' for available commands.",
                            line
                        );
                    }
                }
            }
            Err(rustyline::error::ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    Ok(())
}
