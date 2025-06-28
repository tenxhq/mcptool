mod listtools;
mod ping;

use std::sync::Arc;
use tenx_mcp::auth::{OAuth2Client, OAuth2Config};

use crate::{
    common::connect_to_server, ctx::Ctx, target::Target, utils::TimedFuture, Error, Result,
};

pub use listtools::listtools;
pub use ping::ping;

pub async fn handle_ping_command(
    ctx: &Ctx,
    target: Option<String>,
    auth: Option<String>,
) -> Result<()> {
    let target = resolve_target(ctx, target, &auth)?;
    let (mut client, init_result) = get_client(ctx, &target, auth).await?;

    ctx.output.text(format!("Pinging {target}..."))?;
    ctx.output.text(format!(
        "Server info: {} v{}",
        init_result.server_info.name, init_result.server_info.version
    ))?;

    ping(&mut client, &ctx.output).await
}

pub async fn handle_listtools_command(
    ctx: &Ctx,
    target: Option<String>,
    auth: Option<String>,
) -> Result<()> {
    let target = resolve_target(ctx, target, &auth)?;
    let (mut client, init_result) = get_client(ctx, &target, auth).await?;

    ctx.output.text(format!("Listing tools from {target}..."))?;
    ctx.output.text(format!(
        "Connected to: {} v{}\n",
        init_result.server_info.name, init_result.server_info.version
    ))?;

    listtools(&mut client, &ctx.output).await
}

async fn get_client(
    ctx: &Ctx,
    target: &Target,
    auth: Option<String>,
) -> Result<(tenx_mcp::Client<()>, tenx_mcp::schema::InitializeResult)> {
    if let Some(auth_name) = auth {
        connect_with_auth(ctx, target, &auth_name).await
    } else {
        connect_to_server(target)
            .timed("Connected and initialized")
            .await
    }
}

fn resolve_target(ctx: &Ctx, target: Option<String>, auth_name: &Option<String>) -> Result<Target> {
    match (target, auth_name) {
        (Some(t), _) => {
            // Target provided, parse it
            Ok(Target::parse(&t)?)
        }
        (None, Some(auth)) => {
            // No target but auth provided, get URL from auth
            let storage = ctx.storage()?;
            let auth_entry = storage.get_auth(auth)?;

            ctx.output.text(format!(
                "Using server URL from auth '{}': {}",
                auth, auth_entry.server_url
            ))?;

            Ok(Target::parse(&auth_entry.server_url)?)
        }
        (None, None) => Err(Error::Other(
            "No target specified. Either provide a target URL or use --auth".to_string(),
        )),
    }
}

pub async fn connect_with_auth(
    ctx: &Ctx,
    target: &Target,
    auth_name: &str,
) -> Result<(tenx_mcp::Client<()>, tenx_mcp::schema::InitializeResult)> {
    // Only HTTP/HTTPS targets support OAuth
    match target {
        Target::Http { .. } | Target::Https { .. } => {}
        _ => {
            return Err(Error::Other(
                "OAuth authentication is only supported for HTTP/HTTPS targets".to_string(),
            ))
        }
    }

    // Load auth credentials
    let storage = ctx.storage()?;
    let auth = storage.get_auth(auth_name)?;

    ctx.output
        .text(format!("Using authentication: {auth_name}"))?;

    // Check if token is expired
    if let Some(expires_at) = auth.expires_at {
        if expires_at <= std::time::SystemTime::now() {
            ctx.output
                .warn("Access token has expired. Token refresh not yet implemented.")?;
            return Err(Error::Other(
                "Access token has expired. Please re-authenticate with 'mcptool auth add'"
                    .to_string(),
            ));
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
    let mut client = tenx_mcp::Client::new("mcptool", crate::ctx::VERSION)
        .with_capabilities(tenx_mcp::schema::ClientCapabilities::default());

    // Connect with OAuth
    let init_result = match target {
        Target::Http { host, port } => {
            let url = format!("http://{host}:{port}");
            client
                .connect_http_with_oauth(&url, oauth_client)
                .await
                .map_err(|e| {
                    Error::Other(format!(
                        "Failed to connect to HTTP endpoint {url} with OAuth: {e}"
                    ))
                })?
        }
        Target::Https { host, port } => {
            let url = format!("https://{host}:{port}");
            client
                .connect_http_with_oauth(&url, oauth_client)
                .await
                .map_err(|e| {
                    Error::Other(format!(
                        "Failed to connect to HTTPS endpoint {url} with OAuth: {e}"
                    ))
                })?
        }
        _ => unreachable!(), // We checked this above
    };

    Ok((client, init_result))
}
