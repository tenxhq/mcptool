use rustyline::DefaultEditor;

use crate::{Result, client, ctx::Ctx, mcp, target::Target};

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
                            .text("  ping      - Send a ping request to the server")?;
                        ctx.output
                            .text("  listtools - List all available tools from the server")?;
                        ctx.output.text("  help      - Show this help message")?;
                        ctx.output.text("  quit/exit - Exit the REPL")?
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
