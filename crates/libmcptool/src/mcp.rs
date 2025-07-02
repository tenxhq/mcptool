use crate::{Result, calltool, output, utils::TimedFuture};
use tenx_mcp::{Client, ServerAPI, schema::InitializeResult};

pub async fn ping(client: &mut Client<()>, output: &crate::output::Output) -> Result<()> {
    output.text("Pinging")?;
    client.ping().timed("   response", output).await?;
    output.ping()?;
    Ok(())
}

pub async fn listtools(client: &mut Client<()>, output: &crate::output::Output) -> Result<()> {
    output.text("Listing tools")?;
    let tools_result = client
        .list_tools(None)
        .timed("    response", output)
        .await?;
    output::listtools::list_tools_result(output, &tools_result)?;
    Ok(())
}

pub fn init(init_result: &InitializeResult, output: &crate::output::Output) -> Result<()> {
    output::initresult::init_result(output, init_result)?;
    Ok(())
}

pub async fn listresources(client: &mut Client<()>, output: &crate::output::Output) -> Result<()> {
    output.text("Listing resources")?;
    let resources_result = client
        .list_resources(None)
        .timed("    response", output)
        .await?;
    output::listresources::list_resources_result(output, &resources_result)?;
    Ok(())
}

pub async fn listprompts(client: &mut Client<()>, output: &crate::output::Output) -> Result<()> {
    output.text("Listing prompts")?;
    let prompts_result = client
        .list_prompts(None)
        .timed("    response", output)
        .await?;
    output::listprompts::list_prompts_result(output, &prompts_result)?;
    Ok(())
}

pub async fn listresourcetemplates(
    client: &mut Client<()>,
    output: &crate::output::Output,
) -> Result<()> {
    output.text("Listing resource templates")?;
    let templates_result = client
        .list_resource_templates(None)
        .timed("    response", output)
        .await?;
    output::listresourcetemplates::list_resource_templates_result(output, &templates_result)?;
    Ok(())
}

pub async fn set_level(
    _client: &mut Client<()>,
    output: &crate::output::Output,
    level: &str,
) -> Result<()> {
    output.text(format!("Setting logging level to: {level}"))?;

    // Parse the level string into LoggingLevel enum
    // TODO: Use the proper LoggingLevel type once we figure out the import
    // For now, we'll just validate the level string
    let valid_levels = [
        "debug",
        "info",
        "notice",
        "warning",
        "error",
        "critical",
        "alert",
        "emergency",
    ];
    if !valid_levels.contains(&level.to_lowercase().as_str()) {
        return Err(crate::Error::Other(format!(
            "Invalid logging level: {}. Valid levels are: {}",
            level,
            valid_levels.join(", ")
        )));
    }

    // For now, just print a message since we can't import LoggingLevel
    output
        .trace_warn("set_level is not yet implemented - LoggingLevel type needs to be imported")?;
    output.trace_success(format!("Would set logging level to: {level}"))?;
    Ok(())
}

pub async fn calltool(
    client: &mut Client<()>,
    output: &crate::output::Output,
    tool_name: &str,
    args: Vec<String>,
    interactive: bool,
    json: bool,
) -> Result<()> {
    // Validate input modes
    let mode_count = [!args.is_empty(), interactive, json]
        .iter()
        .filter(|&&x| x)
        .count();
    if mode_count == 0 {
        return Err(crate::Error::Other(
            "Must specify one of: --interactive, --json, or --arg key=value arguments".to_string(),
        ));
    }
    if mode_count > 1 {
        return Err(crate::Error::Other(
            "Cannot combine --interactive, --json, and --arg modes".to_string(),
        ));
    }

    output.text(format!("Calling tool: {tool_name}"))?;

    // First, get tool schema to understand required parameters
    let tools_result = client
        .list_tools(None)
        .timed("   fetching tools", output)
        .await?;

    let tool = tools_result
        .tools
        .iter()
        .find(|t| t.name == tool_name)
        .ok_or_else(|| crate::Error::Other(format!("Tool '{tool_name}' not found")))?;

    // Parse arguments based on mode
    let arguments = if json {
        calltool::json::parse_json_arguments(output)?
    } else if interactive {
        calltool::interactive::parse_interactive_arguments(tool, output)?
    } else {
        calltool::cmdline::parse_command_line_arguments(args, output)?
    };

    // Call the tool
    let result = client
        .call_tool(tool_name, arguments)
        .timed("   response", output)
        .await?;

    output::calltool::call_tool_result(output, &result)
}
