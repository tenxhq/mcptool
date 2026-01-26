use std::{
    net::TcpListener,
    time::{Duration, Instant, SystemTime},
};

use rustyline::DefaultEditor;
use tmcp::auth::{
    AuthorizationServerMetadata, ClientMetadata, DynamicRegistrationClient, OAuth2CallbackServer,
    OAuth2Client, OAuth2Config, OAuth2Token,
};
use tokio::{signal, time::timeout};

use super::discovery::{discover_metadata, display_metadata, get_authorization_base_url};
use crate::{
    Error, Result, auth::validate_auth_name, ctx::Ctx, output::Output, storage::StoredAuth,
};

/// Arguments for the add command.
pub struct AddCommandArgs {
    /// Name for the authentication entry.
    pub name: String,
    /// Server URL (e.g., <https://api.example.com>).
    pub server_url: Option<String>,
    /// OAuth authorization URL.
    pub auth_url: Option<String>,
    /// OAuth token URL.
    pub token_url: Option<String>,
    /// OAuth client ID.
    pub client_id: Option<String>,
    /// OAuth client secret.
    pub client_secret: Option<String>,
    /// OAuth redirect URL (if not provided, will use local server).
    pub redirect_url: Option<String>,
    /// Resource/Audience parameter for OAuth.
    pub resource: Option<String>,
    /// OAuth scopes (comma-separated).
    pub scopes: Option<String>,
    /// Show the redirect URL that will be used without starting OAuth flow.
    pub show_redirect_url: bool,
    /// Skip metadata discovery and use manual configuration.
    pub no_discover: bool,
    /// Automatically register a dynamic client if supported (skip Y/n prompt).
    pub auto_register: bool,
}

/// Resolved OAuth configuration after discovery and/or manual input.
struct ResolvedConfig {
    /// Server URL.
    server_url: String,
    /// Authorization endpoint URL.
    auth_url: String,
    /// Token endpoint URL.
    token_url: String,
    /// Client ID.
    client_id: String,
    /// Client secret (optional).
    client_secret: Option<String>,
    /// Redirect URL for OAuth callback.
    redirect_url: String,
    /// Whether to use local callback server.
    use_local_server: Option<u16>,
    /// Resource (audience) parameter.
    resource: String,
    /// Requested scopes.
    scopes: Vec<String>,
}

/// Adds a new OAuth authentication entry.
pub async fn add_command(ctx: &Ctx, args: AddCommandArgs) -> Result<()> {
    let name = args.name.clone();

    // Validate auth name
    validate_auth_name(&name)?;

    ctx.output
        .h1(format!("Adding OAuth authentication entry: {name}"))?;

    // Check if entry already exists
    let storage = ctx.storage()?;
    if storage.list_auth()?.contains(&name) {
        return Err(Error::Other(format!(
            "Authentication entry '{name}' already exists"
        )));
    }

    // Resolve configuration (with or without discovery)
    let config = resolve_config(ctx, &args).await?;

    // If user just wants to see the redirect URL, show it and exit
    if args.show_redirect_url {
        ctx.output.text("")?;
        ctx.output.h1("OAuth Redirect URL Information")?;
        ctx.output.text(format!(
            "Redirect URL that will be used: {}",
            config.redirect_url
        ))?;
        ctx.output.text("")?;
        ctx.output
            .text("Add this URL to your OAuth application settings.")?;
        ctx.output
            .text("Then run the command again without --show-redirect-url to complete setup.")?;
        return Ok(());
    }

    // Perform OAuth flow
    let token = perform_oauth_flow(ctx, &config).await?;

    // Store the authentication
    store_auth_entry(ctx, &name, &config, token)?;

    ctx.output.text("")?;
    ctx.output
        .trace_success(format!("Authentication entry '{name}' saved successfully!"))?;
    ctx.output.text(format!(
        "You can now use: mcptool connect --auth {name} <target>"
    ))?;

    Ok(())
}

/// Resolves OAuth configuration, using discovery if available.
async fn resolve_config(ctx: &Ctx, args: &AddCommandArgs) -> Result<ResolvedConfig> {
    let mut rl = DefaultEditor::new()?;

    // Get server URL first (always needed)
    let server_url = match &args.server_url {
        Some(url) => url.clone(),
        None => {
            ctx.output.text("Enter the OAuth provider configuration:")?;
            ctx.output.text("")?;
            rl.readline("Server URL (e.g., https://mcp.linear.app/mcp): ")?
        }
    };

    // Try metadata discovery unless disabled
    let metadata = if args.no_discover {
        ctx.output
            .text("Skipping metadata discovery (--no-discover)")?;
        None
    } else {
        ctx.output.text("")?;
        ctx.output.text("Attempting OAuth metadata discovery...")?;
        match discover_metadata(&server_url).await {
            Ok(meta) => {
                ctx.output
                    .trace_success("Metadata discovered successfully!")?;
                ctx.output.text("")?;
                display_metadata(ctx, &meta)?;
                Some(meta)
            }
            Err(e) => {
                ctx.output.trace_warn(format!(
                    "Metadata discovery failed: {e}. Falling back to manual configuration."
                ))?;
                None
            }
        }
    };

    // Get base URL for fallback endpoints
    let base_url = get_authorization_base_url(&server_url)?;

    // Resolve auth and token URLs
    let (auth_url, token_url) = resolve_endpoints(ctx, args, &metadata, &base_url, &mut rl)?;

    // Resolve redirect URL
    let (redirect_url, use_local_server) = resolve_redirect_url(ctx, args)?;

    // Resolve client credentials (with dynamic registration if available)
    let (client_id, client_secret) =
        resolve_client_credentials(ctx, args, &metadata, &base_url, &redirect_url, &mut rl).await?;

    // Resolve resource and scopes
    let resource = args.resource.clone().unwrap_or_else(|| server_url.clone());
    let scopes = resolve_scopes(args, &metadata);

    Ok(ResolvedConfig {
        server_url,
        auth_url,
        token_url,
        client_id,
        client_secret,
        redirect_url,
        use_local_server,
        resource,
        scopes,
    })
}

/// Resolves authorization and token endpoints from metadata or user input.
fn resolve_endpoints(
    ctx: &Ctx,
    args: &AddCommandArgs,
    metadata: &Option<AuthorizationServerMetadata>,
    _base_url: &str,
    rl: &mut DefaultEditor,
) -> Result<(String, String)> {
    let auth_url = if let Some(url) = &args.auth_url {
        url.clone()
    } else if let Some(meta) = metadata
        && let Some(url) = &meta.authorization_endpoint
    {
        ctx.output.kv("Using Authorization Endpoint", url)?;
        url.clone()
    } else {
        rl.readline("Authorization URL: ")?
    };

    let token_url = if let Some(url) = &args.token_url {
        url.clone()
    } else if let Some(meta) = metadata
        && let Some(url) = &meta.token_endpoint
    {
        ctx.output.kv("Using Token Endpoint", url)?;
        url.clone()
    } else {
        rl.readline("Token URL: ")?
    };

    Ok((auth_url, token_url))
}

/// Resolves the redirect URL, potentially setting up a local callback server.
fn resolve_redirect_url(ctx: &Ctx, args: &AddCommandArgs) -> Result<(String, Option<u16>)> {
    match &args.redirect_url {
        Some(url) => Ok((url.clone(), None)),
        None => {
            let callback_port = 8080;
            let actual_port = match TcpListener::bind(format!("127.0.0.1:{callback_port}")) {
                Ok(_) => callback_port,
                Err(_) => find_available_port()?,
            };
            let url = format!("http://127.0.0.1:{actual_port}/callback");
            ctx.output.text(format!("Using redirect URL: {url}"))?;
            ctx.output.trace_warn(
                "Note: This URL must be registered in your OAuth application settings!",
            )?;
            Ok((url, Some(actual_port)))
        }
    }
}

/// Resolves client credentials, attempting dynamic registration if available.
async fn resolve_client_credentials(
    ctx: &Ctx,
    args: &AddCommandArgs,
    metadata: &Option<AuthorizationServerMetadata>,
    base_url: &str,
    redirect_url: &str,
    rl: &mut DefaultEditor,
) -> Result<(String, Option<String>)> {
    // If client_id is provided, use it directly
    if let Some(client_id) = &args.client_id {
        let client_secret = resolve_client_secret(args, rl)?;
        return Ok((client_id.clone(), client_secret));
    }

    // Try dynamic registration if metadata indicates support
    if let Some(meta) = metadata
        && meta.registration_endpoint.is_some()
    {
        ctx.output.text("")?;
        ctx.output.h2("Dynamic Client Registration")?;
        let should_register = if args.auto_register {
            ctx.output
                .text("Automatically registering client (--register flag)")?;
            true
        } else {
            ctx.output
                .text("This server supports dynamic client registration.")?;
            ctx.output
                .text("Would you like to automatically register a client? (Y/n)")?;

            let response = rl.readline("> ")?;
            response.trim().is_empty() || response.trim().to_lowercase().starts_with('y')
        };

        if should_register {
            match perform_dynamic_registration(ctx, meta, base_url, redirect_url).await {
                Ok((client_id, client_secret)) => {
                    ctx.output
                        .trace_success("Client registered successfully!")?;
                    ctx.output.kv("Client ID", &client_id)?;
                    if client_secret.is_some() {
                        ctx.output.kv("Client Secret", "(obtained)")?;
                    }
                    return Ok((client_id, client_secret));
                }
                Err(e) => {
                    ctx.output.trace_warn(format!(
                        "Dynamic registration failed: {e}. Falling back to manual entry."
                    ))?;
                }
            }
        }
    }

    // Fall back to manual entry
    let client_id = rl.readline("Client ID: ")?;
    let client_secret = resolve_client_secret(args, rl)?;
    Ok((client_id, client_secret))
}

/// Resolves the client secret from args or prompts the user.
fn resolve_client_secret(args: &AddCommandArgs, rl: &mut DefaultEditor) -> Result<Option<String>> {
    match &args.client_secret {
        Some(secret) => Ok(Some(secret.clone())),
        None => {
            let input = rl.readline("Client Secret (optional, press Enter to skip): ")?;
            if input.trim().is_empty() {
                Ok(None)
            } else {
                Ok(Some(input))
            }
        }
    }
}

/// Performs dynamic client registration.
async fn perform_dynamic_registration(
    ctx: &Ctx,
    metadata: &AuthorizationServerMetadata,
    _base_url: &str,
    redirect_url: &str,
) -> Result<(String, Option<String>)> {
    let registration_endpoint = metadata
        .registration_endpoint
        .as_ref()
        .ok_or_else(|| Error::Other("No registration endpoint available".to_string()))?;

    ctx.output
        .text(format!("Registering client at: {registration_endpoint}"))?;

    let client_metadata = ClientMetadata::new("mcptool", redirect_url)
        .with_software_info("mcptool", env!("CARGO_PKG_VERSION"));

    let reg_client = DynamicRegistrationClient::new();
    let response = reg_client
        .register(registration_endpoint, client_metadata, None)
        .await
        .map_err(|e| Error::Other(format!("Dynamic registration failed: {e}")))?;

    Ok((response.client_id, response.client_secret))
}

/// Resolves scopes from args or metadata.
fn resolve_scopes(
    args: &AddCommandArgs,
    metadata: &Option<AuthorizationServerMetadata>,
) -> Vec<String> {
    if let Some(scopes_str) = &args.scopes {
        scopes_str
            .split(',')
            .map(|s| s.trim().to_string())
            .collect()
    } else if let Some(meta) = metadata
        && let Some(supported) = &meta.scopes_supported
        && !supported.is_empty()
    {
        // Don't auto-select scopes, just leave empty
        vec![]
    } else {
        vec![]
    }
}

/// Performs the OAuth authorization flow.
async fn perform_oauth_flow(ctx: &Ctx, config: &ResolvedConfig) -> Result<OAuth2Token> {
    ctx.output.text("")?;
    ctx.output.text("Initiating OAuth flow...")?;

    // Create OAuth configuration
    let oauth_config = OAuth2Config {
        client_id: config.client_id.clone(),
        client_secret: config.client_secret.clone(),
        auth_url: config.auth_url.clone(),
        token_url: config.token_url.clone(),
        redirect_url: config.redirect_url.clone(),
        resource: config.resource.clone(),
        scopes: config.scopes.clone(),
    };

    // Create OAuth client
    let mut oauth_client = OAuth2Client::new(oauth_config)?;

    // Get authorization URL
    let (auth_url_with_params, csrf_token) = oauth_client.get_authorization_url();

    ctx.output.text("")?;
    ctx.output.h1("Authorization required")?;
    ctx.output
        .text("Please visit the following URL to authorize the application:")?;
    ctx.output.text("")?;
    ctx.output.text(auth_url_with_params.as_str())?;
    ctx.output.text("")?;

    // Try to open browser
    match open::that(auth_url_with_params.as_str()) {
        Ok(()) => {
            ctx.output.trace_success("Browser opened successfully.")?;
            ctx.output
                .text("If the browser didn't open, copy the URL above and paste it manually.")?;
        }
        Err(e) => {
            ctx.output
                .trace_warn(format!("Could not open browser automatically: {e}"))?;
            ctx.output
                .text("Please copy the URL above and open it manually in your browser.")?;

            #[cfg(target_os = "macos")]
            ctx.output.text(
                "On macOS: You may need to allow the terminal app to control other applications.",
            )?;
        }
    }

    ctx.output.text("")?;

    // Handle different callback modes
    let token_result = if let Some(callback_port) = config.use_local_server {
        let callback_server = OAuth2CallbackServer::new(callback_port);

        ctx.output.text("Waiting for authorization callback...")?;
        ctx.output.text(format!(
            "Local callback server listening on port {callback_port}"
        ))?;
        ctx.output
            .text("The browser will redirect back to this server after authorization.")?;
        ctx.output
            .text("Press Ctrl+C to cancel and use manual mode instead.")?;

        tokio::select! {
            result = wait_for_callback(&mut oauth_client, callback_server, csrf_token.secret().to_string()) => {
                Ok(result)
            }
            () = async { signal::ctrl_c().await.ok(); } => {
                ctx.output.text("")?;
                ctx.output.trace_warn("Cancelled! Switching to manual mode...")?;
                timeout(
                    Duration::from_secs(300),
                    wait_for_manual_callback(&mut oauth_client, csrf_token.secret().to_string(), &ctx.output),
                ).await
            }
        }
    } else {
        ctx.output.text("Manual callback mode:")?;
        ctx.output
            .text("After authorizing, you'll be redirected to your registered URL.")?;
        ctx.output
            .text("Copy the full URL from your browser and paste it when prompted.")?;

        timeout(
            Duration::from_secs(300),
            wait_for_manual_callback(
                &mut oauth_client,
                csrf_token.secret().to_string(),
                &ctx.output,
            ),
        )
        .await
    };

    match token_result {
        Ok(Ok(token)) => {
            ctx.output.trace_success("Authorization successful!")?;
            Ok(token)
        }
        Ok(Err(e)) => {
            let error_msg = format!("{e}");
            handle_oauth_error(ctx, &error_msg, &config.redirect_url)?;
            Err(Error::Other(format!("OAuth error: {error_msg}")))
        }
        Err(_) => Err(Error::Other(
            "OAuth authorization timed out after 5 minutes".to_string(),
        )),
    }
}

/// Handles OAuth errors with helpful error messages.
fn handle_oauth_error(ctx: &Ctx, error_msg: &str, redirect_url: &str) -> Result<()> {
    if error_msg.contains("redirect_uri") || error_msg.contains("redirect URL") {
        ctx.output.text("")?;
        ctx.output
            .trace_error("OAuth Error: Redirect URL not registered")?;
        ctx.output
            .text("The redirect URL is not associated with your OAuth application.")?;
        ctx.output.text("")?;
        ctx.output.text("To fix this:")?;
        ctx.output
            .text("1. Go to your OAuth application settings")?;
        ctx.output
            .text(format!("2. Add this redirect URL: {redirect_url}"))?;
        ctx.output.text("3. Run this command again")?;
    } else if error_msg.contains("incorrect_client_credentials")
        || error_msg.contains("client_id and/or client_secret")
    {
        ctx.output.text("")?;
        ctx.output
            .trace_error("OAuth Error: Invalid client credentials")?;
        ctx.output
            .text("The client_id and/or client_secret are incorrect.")?;
        ctx.output.text("")?;
        ctx.output.text("To fix this:")?;
        ctx.output
            .text("1. Verify your OAuth application settings")?;
        ctx.output
            .text("2. Make sure the client_id and client_secret match exactly")?;
        ctx.output
            .text("3. For GitHub: client_secret is required for OAuth Apps")?;
        ctx.output
            .text("4. Check for trailing spaces or incorrect copy/paste")?;
    }
    ctx.output.text("")?;
    Ok(())
}

/// Stores the authentication entry.
fn store_auth_entry(
    ctx: &Ctx,
    name: &str,
    config: &ResolvedConfig,
    token: OAuth2Token,
) -> Result<()> {
    let storage = ctx.storage()?;

    let expires_at = token.expires_at.map(|instant| {
        let duration_since_now = instant.duration_since(Instant::now());
        SystemTime::now() + duration_since_now
    });

    let stored_auth = StoredAuth {
        name: name.to_string(),
        server_url: config.server_url.clone(),
        client_id: config.client_id.clone(),
        client_secret: config.client_secret.clone(),
        access_token: Some(token.access_token),
        refresh_token: token.refresh_token,
        expires_at,
        auth_url: config.auth_url.clone(),
        token_url: config.token_url.clone(),
        redirect_url: Some(config.redirect_url.clone()),
        scopes: config.scopes.clone(),
    };

    storage.store_auth(&stored_auth)?;
    Ok(())
}

/// Waits for the OAuth callback to be received via local server.
async fn wait_for_callback(
    oauth_client: &mut OAuth2Client,
    callback_server: OAuth2CallbackServer,
    expected_state: String,
) -> Result<OAuth2Token> {
    // Wait for the OAuth callback
    let (code, state) = callback_server.wait_for_callback().await?;

    // Verify the state parameter matches for CSRF protection
    if state != expected_state {
        return Err(Error::Other(
            "State parameter mismatch - possible CSRF attack".to_string(),
        ));
    }

    // Exchange the authorization code for an access token
    let token = oauth_client.exchange_code(code, state).await?;

    Ok(token)
}

/// Waits for the OAuth callback URL to be manually entered by the user.
async fn wait_for_manual_callback(
    oauth_client: &mut OAuth2Client,
    expected_state: String,
    output: &Output,
) -> Result<OAuth2Token> {
    let mut rl = DefaultEditor::new()?;

    output.text("")?;
    let callback_url = rl.readline("Paste the full callback URL from your browser: ")?;

    // Extract the authorization code and state from the callback URL
    let url = url::Url::parse(&callback_url)
        .map_err(|e| Error::Other(format!("Invalid URL format: {e}")))?;

    let mut code = None;
    let mut state = None;
    let mut error = None;
    let mut error_description = None;

    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "code" => code = Some(value.to_string()),
            "state" => state = Some(value.to_string()),
            "error" => error = Some(value.to_string()),
            "error_description" => error_description = Some(value.to_string()),
            _ => {}
        }
    }

    // Check for OAuth errors first
    if let Some(error_code) = error {
        let description =
            error_description.unwrap_or_else(|| "No description provided".to_string());
        return Err(Error::Other(format!(
            "OAuth authorization failed: {error_code} - {description}"
        )));
    }

    let code = code.ok_or(Error::Other(
        "No authorization code found in callback URL".to_string(),
    ))?;
    let state = state.ok_or(Error::Other(
        "No state parameter found in callback URL".to_string(),
    ))?;

    // Verify the state parameter matches for CSRF protection
    if state != expected_state {
        return Err(Error::Other(
            "State parameter mismatch - possible CSRF attack".to_string(),
        ));
    }

    // Exchange the authorization code for an access token
    let token = oauth_client
        .exchange_code(code, state)
        .await
        .map_err(|e| Error::Other(format!("Token exchange failed: {e}")))?;

    Ok(token)
}

/// Finds an available port on localhost.
fn find_available_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    Ok(addr.port())
}

#[cfg(test)]
mod tests {
    use crate::auth::validate_auth_name;

    #[test]
    fn test_validate_auth_name_valid() {
        // Valid names
        assert!(validate_auth_name("myauth").is_ok());
        assert!(validate_auth_name("my_auth").is_ok());
        assert!(validate_auth_name("MyAuth123").is_ok());
        assert!(validate_auth_name("AUTH_123_test").is_ok());
        assert!(validate_auth_name("a").is_ok());
        assert!(validate_auth_name("_").is_ok());
        assert!(validate_auth_name("123").is_ok());
    }

    #[test]
    fn test_validate_auth_name_invalid_chars_1() {
        assert!(validate_auth_name("my-auth").is_err());
        assert!(validate_auth_name("my auth").is_err());
        assert!(validate_auth_name("my:auth").is_err());
        assert!(validate_auth_name("my/auth").is_err());
        assert!(validate_auth_name("my.auth").is_err());
        assert!(validate_auth_name("my@auth").is_err());
        assert!(validate_auth_name("my#auth").is_err());
        assert!(validate_auth_name("my$auth").is_err());
        assert!(validate_auth_name("my%auth").is_err());
        assert!(validate_auth_name("my^auth").is_err());
        assert!(validate_auth_name("my&auth").is_err());
        assert!(validate_auth_name("my*auth").is_err());
        assert!(validate_auth_name("my(auth)").is_err());
        assert!(validate_auth_name("my[auth]").is_err());
    }

    #[test]
    fn test_validate_auth_name_invalid_chars_2() {
        assert!(validate_auth_name("my{auth}").is_err());
        assert!(validate_auth_name("my|auth").is_err());
        assert!(validate_auth_name("my\\auth").is_err());
        assert!(validate_auth_name("my;auth").is_err());
        assert!(validate_auth_name("my'auth").is_err());
        assert!(validate_auth_name("my\"auth").is_err());
        assert!(validate_auth_name("my<auth>").is_err());
        assert!(validate_auth_name("my?auth").is_err());
        assert!(validate_auth_name("my!auth").is_err());
        assert!(validate_auth_name("my~auth").is_err());
        assert!(validate_auth_name("my`auth").is_err());
        assert!(validate_auth_name("my+auth").is_err());
        assert!(validate_auth_name("my=auth").is_err());
        assert!(validate_auth_name("my,auth").is_err());
    }

    #[test]
    fn test_validate_auth_name_invalid_misc() {
        assert!(validate_auth_name("").is_err());
        // Unicode characters should be invalid
        assert!(validate_auth_name("myαuth").is_err());
        assert!(validate_auth_name("my😀auth").is_err());
        assert!(validate_auth_name("mÿauth").is_err());
    }
}
