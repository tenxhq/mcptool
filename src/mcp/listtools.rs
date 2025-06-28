use crate::core::MCPTool;
use crate::common::connect_to_server;
use crate::output::Output;
use crate::target::Target;
use crate::utils::TimedFuture;
use tenx_mcp::{Client, ServerAPI};

pub async fn listtools_command(
    target: Target,
    auth: Option<String>,
    mcptool: &MCPTool,
    output: Output,
) -> Result<(), Box<dyn std::error::Error>> {
    output.text(format!("Listing tools from {target}..."))?;

    let (mut client, init_result) = if let Some(auth_name) = auth {
        super::connect_with_auth(&target, &auth_name, mcptool, &output).await?
    } else {
        connect_to_server(&target).await?
    };

    output.text(format!(
        "Connected to: {} v{}\n",
        init_result.server_info.name, init_result.server_info.version
    ))?;

    execute_listtools(&mut client, &output).await?;

    Ok(())
}

async fn execute_listtools(
    client: &mut Client<()>,
    output: &Output,
) -> Result<(), Box<dyn std::error::Error>> {
    let tools_result = client.list_tools(None).timed("Tools retrieved").await?;
    display_tools(&tools_result, output)?;
    Ok(())
}

fn display_tools(
    tools_result: &tenx_mcp::schema::ListToolsResult,
    output: &Output,
) -> Result<(), Box<dyn std::error::Error>> {
    if tools_result.tools.is_empty() {
        output.text("No tools available from this server.")?;
    } else {
        output.heading(format!("Available tools ({}):", tools_result.tools.len()))?;
        output.text("")?;
        for tool in &tools_result.tools {
            output.text(format!("  - {}", tool.name))?;

            output.text("")?;
            output.text("    Description:")?;
            output.text("")?;
            match &tool.description {
                Some(description) => {
                    for line in description.lines() {
                        output.text(format!("      {line}"))?;
                    }
                }
                None => output.text("      No description available")?,
            }

            output.text("")?;
            output.text("    Annotations:")?;
            output.text("")?;
            match &tool.annotations {
                Some(annotations) => {
                    output.text(format!("      {:?}", annotations.title))?;
                }
                None => output.text("      No annotations available")?,
            }

            output.text("")?;
            output.text("    Input arguments:")?;
            output.text("")?;

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
                        output.text(format!("      {name} - (required: {is_required})\n"))?;

                        for line in rendered_schema.lines() {
                            output.text(format!("        {line}"))?;
                        }
                        output.text("")?;
                    }
                }
                None => output.text("      No input schema available")?,
            }

            output.text("")?; // Extra blank line between tools
        }
    }
    Ok(())
}
