use rustyline::DefaultEditor;

use crate::{Result, client, ctx::Ctx, mcp, output::initresult, target::Target};

/// Parse command arguments that support both --arg key=value and direct key=value formats
fn parse_command_args(parts: &[&str], output: &crate::output::Output) -> Result<Vec<String>> {
    let mut args: Vec<String> = Vec::new();
    let mut i = 1;

    // Parse arguments, handling --arg prefix
    while i < parts.len() {
        let part = parts[i];
        if part == "--arg" {
            // Next part should be key=value
            if i + 1 < parts.len() {
                args.push(parts[i + 1].to_string());
                i += 2;
            } else {
                output.trace_error("--arg requires a key=value argument")?;
                break;
            }
        } else if part.contains('=') {
            // Direct key=value without --arg prefix
            args.push(part.to_string());
            i += 1;
        } else {
            output.trace_error(format!(
                "Invalid argument: {}. Use --arg key=value or key=value",
                part
            ))?;
            break;
        }
    }

    Ok(args)
}

pub async fn connect_command(ctx: &Ctx, target: String) -> Result<()> {
    let target = Target::parse(&target)?;

    ctx.output.text(format!("Connecting to {target}..."))?;

    let (mut client, init_result) = client::get_client(ctx, &target).await?;

    ctx.output.trace_success(format!(
        "Connected to: {} v{}",
        init_result.server_info.name, init_result.server_info.version
    ))?;
    ctx.output
        .text("Type 'help' for available commands, 'quit' to exit\n")?;

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
                        ctx.output.text("Goodbye!")?;
                        break;
                    }
                    "help" => {
                        ctx.output.h1("Available commands")?;
                        ctx.output
                            .text("  init          - Display server initialization information")?;
                        ctx.output
                            .text("  ping          - Send a ping request to the server")?;
                        ctx.output
                            .text("  listtools     - List all available tools from the server")?;
                        ctx.output.text(
                            "  listresources - List all available resources from the server",
                        )?;
                        ctx.output
                            .text("  listprompts   - List all available prompts from the server")?;
                        ctx.output.text(
                            "  listresourcetemplates - List all available resource templates from the server",
                        )?;
                        ctx.output
                            .text("  setlevel <level> - Set the logging level on the server")?;
                        ctx.output.text(
                            "  calltool <name> [--arg key=value ...] - Call a tool with arguments",
                        )?;
                        ctx.output
                            .text("  readresource <uri> - Read a resource by URI")?;
                        ctx.output
                            .text("  getprompt <name> [--arg key=value ...] - Get a prompt by name with optional arguments")?;
                        ctx.output
                            .text("  subscriberesource <uri> - Subscribe to resource update notifications")?;
                        ctx.output
                            .text("  unsubscriberesource <uri> - Unsubscribe from resource update notifications")?;
                        ctx.output.text(
                            "  complete <reference> <argument> - Get completion suggestions",
                        )?;
                        ctx.output
                            .text("  help          - Show this help message")?;
                        ctx.output.text("  quit/exit     - Exit the REPL")?
                    }
                    "init" => {
                        ctx.output.note("Showing initialization result from initial connection (not re-initializing)")?;
                        initresult::init_result(&ctx.output, &init_result)?;
                    }
                    "ping" => match mcp::ping(&mut client, &ctx.output).await {
                        Ok(_) => {}
                        Err(e) => ctx.output.trace_error(format!("Ping failed: {e}"))?,
                    },
                    "listtools" => match mcp::listtools(&mut client, &ctx.output).await {
                        Ok(_) => {}
                        Err(e) => ctx
                            .output
                            .trace_error(format!("Failed to list tools: {e}"))?,
                    },
                    "listresources" => match mcp::listresources(&mut client, &ctx.output).await {
                        Ok(_) => {}
                        Err(e) => ctx
                            .output
                            .trace_error(format!("Failed to list resources: {e}"))?,
                    },
                    "listprompts" => match mcp::listprompts(&mut client, &ctx.output).await {
                        Ok(_) => {}
                        Err(e) => ctx
                            .output
                            .trace_error(format!("Failed to list prompts: {e}"))?,
                    },
                    "listresourcetemplates" => {
                        match mcp::listresourcetemplates(&mut client, &ctx.output).await {
                            Ok(_) => {}
                            Err(e) => ctx
                                .output
                                .trace_error(format!("Failed to list resource templates: {e}"))?,
                        }
                    }
                    cmd if cmd.starts_with("setlevel ") => {
                        let level = cmd.strip_prefix("setlevel ").unwrap_or("").trim();
                        if level.is_empty() {
                            ctx.output.trace_error("Usage: setlevel <level>")?;
                            ctx.output.text("Valid levels: debug, info, notice, warning, error, critical, alert, emergency")?;
                        } else {
                            match mcp::set_level(&mut client, &ctx.output, level).await {
                                Ok(_) => {}
                                Err(e) => ctx
                                    .output
                                    .trace_error(format!("Failed to set level: {e}"))?,
                            }
                        }
                    }
                    cmd if cmd.starts_with("calltool ") => {
                        let args_str = cmd.strip_prefix("calltool ").unwrap_or("").trim();
                        let parts: Vec<&str> = args_str.split_whitespace().collect();
                        if parts.is_empty() {
                            ctx.output
                                .trace_error("Usage: calltool <name> [--arg key=value ...]")?;
                        } else {
                            let tool_name = parts[0];
                            let args = parse_command_args(&parts, &ctx.output)?;

                            match mcp::calltool(
                                &mut client,
                                &ctx.output,
                                tool_name,
                                args,
                                false,
                                false,
                            )
                            .await
                            {
                                Ok(_) => {}
                                Err(e) => ctx
                                    .output
                                    .trace_error(format!("Failed to call tool: {e}"))?,
                            }
                        }
                    }
                    cmd if cmd.starts_with("readresource ") => {
                        let uri = cmd.strip_prefix("readresource ").unwrap_or("").trim();
                        if uri.is_empty() {
                            ctx.output.trace_error("Usage: readresource <uri>")?;
                        } else {
                            match mcp::read_resource(&mut client, &ctx.output, uri).await {
                                Ok(_) => {}
                                Err(e) => ctx
                                    .output
                                    .trace_error(format!("Failed to read resource: {e}"))?,
                            }
                        }
                    }
                    cmd if cmd.starts_with("getprompt ") => {
                        let args_str = cmd.strip_prefix("getprompt ").unwrap_or("").trim();
                        let parts: Vec<&str> = args_str.split_whitespace().collect();
                        if parts.is_empty() {
                            ctx.output
                                .trace_error("Usage: getprompt <name> [--arg key=value ...]")?;
                        } else {
                            let prompt_name = parts[0];
                            let args = parse_command_args(&parts, &ctx.output)?;

                            match mcp::get_prompt(&mut client, &ctx.output, prompt_name, args).await
                            {
                                Ok(_) => {}
                                Err(e) => ctx
                                    .output
                                    .trace_error(format!("Failed to get prompt: {e}"))?,
                            }
                        }
                    }
                    cmd if cmd.starts_with("subscriberesource ") => {
                        let uri = cmd.strip_prefix("subscriberesource ").unwrap_or("").trim();
                        if uri.is_empty() {
                            ctx.output.trace_error("Usage: subscriberesource <uri>")?;
                        } else {
                            match mcp::subscribe_resource(&mut client, &ctx.output, uri).await {
                                Ok(_) => {}
                                Err(e) => ctx
                                    .output
                                    .trace_error(format!("Failed to subscribe to resource: {e}"))?,
                            }
                        }
                    }
                    cmd if cmd.starts_with("unsubscriberesource ") => {
                        let uri = cmd
                            .strip_prefix("unsubscriberesource ")
                            .unwrap_or("")
                            .trim();
                        if uri.is_empty() {
                            ctx.output.trace_error("Usage: unsubscriberesource <uri>")?;
                        } else {
                            match mcp::unsubscribe_resource(&mut client, &ctx.output, uri).await {
                                Ok(_) => {}
                                Err(e) => ctx.output.trace_error(format!(
                                    "Failed to unsubscribe from resource: {e}"
                                ))?,
                            }
                        }
                    }
                    cmd if cmd.starts_with("complete ") => {
                        let args_str = cmd.strip_prefix("complete ").unwrap_or("").trim();
                        let parts: Vec<&str> = args_str.split_whitespace().collect();
                        if parts.len() < 2 {
                            ctx.output
                                .trace_error("Usage: complete <reference> <argument>")?;
                        } else {
                            let reference = parts[0];
                            let argument = parts[1];
                            match mcp::complete(&mut client, &ctx.output, reference, argument).await
                            {
                                Ok(_) => {}
                                Err(e) => ctx
                                    .output
                                    .trace_error(format!("Failed to get completion: {e}"))?,
                            }
                        }
                    }
                    _ => ctx.output.trace_warn(format!(
                        "Unknown command: {line}. Type 'help' for available commands."
                    ))?,
                }
            }
            Err(rustyline::error::ReadlineError::Interrupted) => {
                ctx.output.text("CTRL-C")?;
                break;
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                ctx.output.text("CTRL-D")?;
                break;
            }
            Err(err) => {
                ctx.output.trace_error(format!("Error: {err:?}"))?;
                break;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_args() {
        let output = crate::output::Output::new(false, 80);

        // Test --arg format
        let parts = vec!["echo", "--arg", "message=hello"];
        let args = parse_command_args(&parts, &output).unwrap();
        assert_eq!(args, vec!["message=hello"]);

        // Test direct key=value format
        let parts = vec!["echo", "message=hello"];
        let args = parse_command_args(&parts, &output).unwrap();
        assert_eq!(args, vec!["message=hello"]);

        // Test mixed formats
        let parts = vec!["echo", "--arg", "message=hello", "flag=true"];
        let args = parse_command_args(&parts, &output).unwrap();
        assert_eq!(args, vec!["message=hello", "flag=true"]);

        // Test multiple --arg
        let parts = vec!["echo", "--arg", "message=hello", "--arg", "flag=true"];
        let args = parse_command_args(&parts, &output).unwrap();
        assert_eq!(args, vec!["message=hello", "flag=true"]);
    }
}
