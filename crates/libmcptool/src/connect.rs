use clap::Parser;
use rustyline::DefaultEditor;

use crate::{
    Result, client,
    command::{ReplCommandWrapper, execute_mcp_command_with_client},
    ctx::Ctx,
    output::initresult,
    target::Target,
};

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
                    _ => {
                        // Try to parse as an MCP command using clap
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        match ReplCommandWrapper::try_parse_from(parts) {
                            Ok(wrapper) => {
                                match execute_mcp_command_with_client(
                                    wrapper.command,
                                    &mut client,
                                    &init_result,
                                    ctx,
                                )
                                .await
                                {
                                    Ok(_) => {}
                                    Err(e) => {
                                        ctx.output.trace_error(format!("Command failed: {e}"))?
                                    }
                                }
                            }
                            Err(e) => {
                                ctx.output.trace_error(format!("Invalid command: {e}"))?;
                                ctx.output.text("Type 'help' for available commands.")?;
                            }
                        }
                    }
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
