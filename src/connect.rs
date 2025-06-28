use crate::common::connect_to_server;
use crate::output::Output;
use crate::storage::TokenStorage;
use crate::target::Target;
use crate::utils::TimedFuture;
use rustyline::DefaultEditor;
use std::sync::Arc;
use tenx_mcp::{
    Client, ServerAPI,
    auth::{OAuth2Client, OAuth2Config},
};

pub async fn connect_command(
    target: Option<String>,
    auth_name: Option<String>,
    output: Output,
) -> Result<(), Box<dyn std::error::Error>> {
    // Determine the target to connect to
    let (final_target, used_auth) = match (target, auth_name) {
        (Some(t), auth) => {
            // Target provided, parse it
            let target = Target::parse(&t)?;
            (target, auth)
        }
        (None, Some(auth)) => {
            // No target but auth provided, get URL from auth
            let storage = TokenStorage::new()?;
            let auth_entry = storage.get_auth(&auth)?;

            output.text(format!(
                "Using server URL from auth '{}': {}",
                auth, auth_entry.server_url
            ))?;

            let target = Target::parse(&auth_entry.server_url)?;
            (target, Some(auth))
        }
        (None, None) => {
            return Err("No target specified. Either provide a target URL or use --auth".into());
        }
    };

    output.text(format!("Connecting to {final_target}..."))?;

    let (mut client, init_result) = if let Some(auth_name) = used_auth {
        connect_with_auth(&final_target, &auth_name, &output).await?
    } else {
        connect_to_server(&final_target).await?
    };

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

async fn connect_with_auth(
    target: &Target,
    auth_name: &str,
    output: &Output,
) -> Result<(Client<()>, tenx_mcp::schema::InitializeResult), Box<dyn std::error::Error>> {
    // Only HTTP/HTTPS targets support OAuth
    match target {
        Target::Http { .. } | Target::Https { .. } => {}
        _ => return Err("OAuth authentication is only supported for HTTP/HTTPS targets".into()),
    }

    // Load auth credentials
    let storage = TokenStorage::new()?;
    let auth = storage.get_auth(auth_name)?;

    output.text(format!("Using authentication: {}", auth_name))?;

    // Check if token is expired
    if let Some(expires_at) = auth.expires_at {
        if expires_at <= std::time::SystemTime::now() {
            output.warn("Access token has expired. Token refresh not yet implemented.")?;
            return Err(
                "Access token has expired. Please re-authenticate with 'mcptool auth add'".into(),
            );
        }
    }

    // Create OAuth config
    let oauth_config = OAuth2Config {
        client_id: auth.client_id,
        client_secret: auth.client_secret,
        auth_url: auth.auth_url,
        token_url: auth.token_url,
        redirect_url: auth
            .redirect_url
            .unwrap_or_else(|| "http://localhost:0".to_string()),
        resource: "".to_string(), // Empty resource, could be stored in auth if needed
        scopes: auth.scopes,
    };

    // Create OAuth client
    let oauth_client = OAuth2Client::new(oauth_config)?;

    // Set the stored tokens if available
    if let Some(access_token) = auth.access_token {
        let token = tenx_mcp::auth::OAuth2Token {
            access_token,
            refresh_token: auth.refresh_token,
            expires_at: auth.expires_at.map(|system_time| {
                // Convert SystemTime to Instant
                match system_time.duration_since(std::time::SystemTime::now()) {
                    Ok(duration) => std::time::Instant::now() + duration,
                    Err(_) => std::time::Instant::now(), // Token is already expired
                }
            }),
        };
        oauth_client.set_token(token).await;
    }

    let oauth_client = Arc::new(oauth_client);

    // Create MCP client
    let mut client = Client::new("mcptool", crate::VERSION)
        .with_capabilities(tenx_mcp::schema::ClientCapabilities::default());

    // Connect with OAuth
    let init_result = match target {
        Target::Http { host, port } => {
            let url = format!("http://{host}:{port}");
            client
                .connect_http_with_oauth(&url, oauth_client)
                .await
                .map_err(|e| format!("Failed to connect to HTTP endpoint {url} with OAuth: {e}"))?
        }
        Target::Https { host, port } => {
            let url = format!("https://{host}:{port}");
            client
                .connect_http_with_oauth(&url, oauth_client)
                .await
                .map_err(|e| format!("Failed to connect to HTTPS endpoint {url} with OAuth: {e}"))?
        }
        _ => unreachable!(), // We checked this above
    };

    Ok((client, init_result))
}
