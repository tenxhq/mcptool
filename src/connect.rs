use crate::common::connect_to_server;
use crate::output::Output;
use crate::target::Target;
use crate::utils::TimedFuture;
use rustyline::DefaultEditor;
use tenx_mcp::{Client, ServerAPI};

pub async fn connect_command(
    target: Target,
    output: Output,
) -> Result<(), Box<dyn std::error::Error>> {
    output.text(format!("Connecting to {target}..."))?;

    let (mut client, init_result) = connect_to_server(&target).await?;

    output.success(format!(
        "Connected to: {} v{}",
        init_result.server_info.name, init_result.server_info.version
    ))?;
    output.text("Type 'help' for available commands, 'quit' to exit\n")?;

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
                        output.text("Goodbye!")?;
                        break;
                    }
                    "help" => {
                        output.heading("Available commands")?;
                        output.text("  ping      - Send a ping request to the server")?;
                        output.text("  listtools - List all available tools from the server")?;
                        output.text("  help      - Show this help message")?;
                        output.text("  quit/exit - Exit the REPL")?;
                    }
                    "ping" => match execute_ping(&mut client).await {
                        Ok(_) => output.success("Ping successful!")?,
                        Err(e) => output.error(format!("Ping failed: {e}"))?,
                    },
                    "listtools" => match execute_listtools(&mut client, &output).await {
                        Ok(_) => {}
                        Err(e) => output.error(format!("Failed to list tools: {e}"))?,
                    },
                    _ => {
                        output.warn(format!(
                            "Unknown command: {line}. Type 'help' for available commands."
                        ))?;
                    }
                }
            }
            Err(rustyline::error::ReadlineError::Interrupted) => {
                output.text("CTRL-C")?;
                break;
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                output.text("CTRL-D")?;
                break;
            }
            Err(err) => {
                output.error(format!("Error: {err:?}"))?;
                break;
            }
        }
    }

    Ok(())
}

async fn execute_ping(client: &mut Client<()>) -> Result<(), Box<dyn std::error::Error>> {
    client.ping().timed("Pinged").await?;
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
                        output.text(format!("      {name} - (required: {is_required})"))?;
                        output.text("")?;

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
