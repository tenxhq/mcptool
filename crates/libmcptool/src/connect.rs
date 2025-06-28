use crate::common::connect_to_server;
use rustyline::DefaultEditor;

use crate::{client, ctx::Ctx, mcp, target::Target, Error, Result};

pub async fn connect_command(
    ctx: &Ctx,
    target: Option<String>,
    auth_name: Option<String>,
) -> Result<()> {
    // Determine the target to connect to
    let (final_target, used_auth) = match (target, auth_name) {
        (Some(t), auth) => {
            // Target provided, parse it
            let target = Target::parse(&t)?;
            (target, auth)
        }
        (None, Some(auth)) => {
            // No target but auth provided, get URL from auth
            let storage = ctx.storage()?;
            let auth_entry = storage.get_auth(&auth)?;

            ctx.output.text(format!(
                "Using server URL from auth '{}': {}",
                auth, auth_entry.server_url
            ))?;

            let target = Target::parse(&auth_entry.server_url)?;
            (target, Some(auth))
        }
        (None, None) => {
            return Err(Error::Other(
                "No target specified. Either provide a target URL or use --auth".to_string(),
            ));
        }
    };

    ctx.output
        .text(format!("Connecting to {final_target}..."))?;

    let (mut client, init_result) = if let Some(auth_name) = used_auth {
        client::connect_with_auth(ctx, &final_target, &auth_name).await?
    } else {
        connect_to_server(&final_target).await?
    };

    ctx.output.success(format!(
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
                        ctx.output.heading("Available commands")?;
                        ctx.output
                            .text("  ping      - Send a ping request to the server")?;
                        ctx.output
                            .text("  listtools - List all available tools from the server")?;
                        ctx.output.text("  help      - Show this help message")?;
                        ctx.output.text("  quit/exit - Exit the REPL")?
                    }
                    "ping" => match mcp::ping(&mut client, &ctx.output).await {
                        Ok(_) => {}
                        Err(e) => ctx.output.error(format!("Ping failed: {e}"))?,
                    },
                    "listtools" => match mcp::listtools(&mut client, &ctx.output).await {
                        Ok(_) => {}
                        Err(e) => ctx.output.error(format!("Failed to list tools: {e}"))?,
                    },
                    _ => ctx.output.warn(format!(
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
                ctx.output.error(format!("Error: {err:?}"))?;
                break;
            }
        }
    }

    Ok(())
}
