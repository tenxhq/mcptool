mod target;

use clap::{Parser, Subcommand};
use target::Target;
use tenx_mcp::{
    Client,
    schema::{ClientCapabilities, Implementation},
};

pub const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "-",
    env!("VERGEN_GIT_SHA"),
    " (",
    env!("VERGEN_BUILD_DATE"),
    ")"
);

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

    /// List all MCP tools from a server
    Listtools {
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

        Commands::Listtools { target } => {
            let target = Target::parse(&target)?;
            listtools_command(target).await?;
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

async fn connect_to_server(
    target: &Target,
) -> Result<(Client, tenx_mcp::schema::InitializeResult), Box<dyn std::error::Error>> {
    let mut client = Client::new();

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

    Ok((client, init_result))
}

async fn ping_once(target: &Target) -> Result<(), Box<dyn std::error::Error>> {
    use std::time::Instant;

    let start_time = Instant::now();
    let connect_start = Instant::now();

    let (mut client, init_result) = connect_to_server(target).await?;
    let connect_duration = connect_start.elapsed();

    println!(
        "Connected and initialized in {:.2}ms",
        connect_duration.as_secs_f64() * 1000.0
    );
    println!(
        "Server info: {} v{}",
        init_result.server_info.name, init_result.server_info.version
    );

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

async fn listtools_command(target: Target) -> Result<(), Box<dyn std::error::Error>> {
    println!("Listing tools from {}...", target);

    let (mut client, init_result) = connect_to_server(&target).await?;

    println!(
        "Connected to: {} v{}\n",
        init_result.server_info.name, init_result.server_info.version
    );

    let tools_result = client
        .list_tools()
        .await
        .map_err(|e| format!("Failed to list tools: {}", e))?;

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
