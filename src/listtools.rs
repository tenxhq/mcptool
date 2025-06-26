use crate::common::connect_to_server;
use crate::target::Target;
use crate::utils::TimedFuture;
use tenx_mcp::{Client, ServerAPI};

pub async fn listtools_command(target: Target) -> Result<(), Box<dyn std::error::Error>> {
    println!("Listing tools from {target}...");

    let (mut client, init_result) = connect_to_server(&target).await?;

    println!(
        "Connected to: {} v{}\n",
        init_result.server_info.name, init_result.server_info.version
    );

    execute_listtools(&mut client).await?;

    Ok(())
}

async fn execute_listtools(client: &mut Client<()>) -> Result<(), Box<dyn std::error::Error>> {
    let tools_result = client.list_tools(None).timed("Tools retrieved").await?;
    display_tools(&tools_result)?;
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
                        println!("      {line}");
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
                            .map_err(|e| format!("Failed to serialize schema: {e}"))?;
                        let is_required = &tool
                            .clone()
                            .input_schema
                            .required
                            .is_some_and(|list| list.contains(name));
                        println!("      {name} - (required: {is_required})\n");

                        for line in rendered_schema.lines() {
                            println!("        {line}");
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

