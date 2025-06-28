use crate::output::Output;
use crate::storage::{StoredAuth, TokenStorage};
use rustyline::DefaultEditor;
use secrecy::SecretString;
use std::time::{Duration, SystemTime};
use tenx_mcp::auth::{OAuth2Client, OAuth2Config};
use tokio::time::timeout;

pub async fn add_command(name: String, output: Output) -> Result<(), Box<dyn std::error::Error>> {
    output.heading(format!("Adding OAuth authentication entry: {}", name))?;

    // Check if entry already exists
    let storage = TokenStorage::new()?;
    if storage.list_auth()?.contains(&name) {
        return Err(format!("Authentication entry '{}' already exists", name).into());
    }

    // Use rustyline for interactive prompts
    let mut rl = DefaultEditor::new()?;

    output.text("Enter the OAuth provider configuration:")?;
    output.text("")?;

    // Collect OAuth configuration
    let server_url = rl.readline("Server URL (e.g., https://api.example.com): ")?;
    let auth_url = rl.readline("Authorization URL: ")?;
    let token_url = rl.readline("Token URL: ")?;
    let client_id = rl.readline("Client ID: ")?;

    // Client secret is optional
    let client_secret_input = rl.readline("Client Secret (optional, press Enter to skip): ")?;
    let client_secret = if client_secret_input.trim().is_empty() {
        None
    } else {
        Some(client_secret_input)
    };

    // Redirect URL with default
    let redirect_url_input = rl.readline("Redirect URL (default: http://localhost:0): ")?;
    let redirect_url = if redirect_url_input.trim().is_empty() {
        "http://localhost:0".to_string()
    } else {
        redirect_url_input
    };

    // Resource (audience) - optional for some providers
    let resource_input = rl.readline("Resource/Audience (optional, press Enter to skip): ")?;
    let resource = if resource_input.trim().is_empty() {
        None
    } else {
        Some(resource_input)
    };

    // Scopes
    let scopes_input = rl.readline("Scopes (space-separated, e.g., 'read write'): ")?;
    let scopes: Vec<String> = if scopes_input.trim().is_empty() {
        vec![]
    } else {
        scopes_input.split_whitespace().map(String::from).collect()
    };

    output.text("")?;
    output.text("Initiating OAuth flow...")?;

    // Create OAuth configuration
    let oauth_config = OAuth2Config {
        client_id: client_id.clone(),
        client_secret: client_secret.clone(),
        auth_url: auth_url.clone(),
        token_url: token_url.clone(),
        redirect_url: redirect_url.clone(),
        resource: resource.unwrap_or_default(),
        scopes: scopes.clone(),
    };

    // Create OAuth client
    let mut oauth_client = OAuth2Client::new(oauth_config)?;

    // Get authorization URL
    let (auth_url_with_params, csrf_token) = oauth_client.get_authorization_url();

    output.text("")?;
    output.heading("Authorization required")?;
    output.text("Please visit the following URL to authorize the application:")?;
    output.text("")?;
    output.text(auth_url_with_params.as_str())?;
    output.text("")?;

    // Try to open browser
    if let Err(e) = open::that(auth_url_with_params.as_str()) {
        output.warn(format!("Could not open browser automatically: {}", e))?;
        output.text("Please open the URL manually in your browser.")?;
    } else {
        output.success("Browser opened successfully.")?;
    }

    output.text("")?;
    output.text("Waiting for authorization callback...")?;

    // Wait for the callback with a timeout
    let token_result = timeout(
        Duration::from_secs(300), // 5 minute timeout
        wait_for_token(&mut oauth_client, csrf_token.secret().to_string()),
    )
    .await;

    let token = match token_result {
        Ok(Ok(token)) => token,
        Ok(Err(e)) => return Err(format!("OAuth error: {}", e).into()),
        Err(_) => return Err("OAuth authorization timed out after 5 minutes".into()),
    };

    output.success("Authorization successful!")?;

    // Convert token expiration from Instant to SystemTime
    let expires_at = token.expires_at.map(|instant| {
        let duration_since_now = instant.duration_since(std::time::Instant::now());
        SystemTime::now() + duration_since_now
    });

    // Store the authentication
    let stored_auth = StoredAuth {
        name: name.clone(),
        server_url,
        client_id,
        client_secret,
        access_token: Some(SecretString::new(token.access_token.into())),
        refresh_token: token.refresh_token,
        expires_at,
        auth_url,
        token_url,
        redirect_url: Some(redirect_url),
        scopes,
    };

    storage.store_auth(&stored_auth)?;

    output.text("")?;
    output.success(format!(
        "Authentication entry '{}' saved successfully!",
        name
    ))?;
    output.text(format!(
        "You can now use: mcptool connect --auth {} <target>",
        name
    ))?;

    Ok(())
}

async fn wait_for_token(
    oauth_client: &mut OAuth2Client,
    _expected_state: String,
) -> Result<tenx_mcp::auth::OAuth2Token, Box<dyn std::error::Error>> {
    // The OAuth2Client handles the callback server internally
    // We just need to wait for the exchange to complete

    // In a real implementation, this would be handled by the OAuth2Client
    // which would start a local server and wait for the callback

    // For now, we'll prompt the user to paste the callback URL
    let mut rl = DefaultEditor::new()?;
    println!();
    let callback_url = rl.readline("After authorizing, paste the full callback URL here: ")?;

    // Extract the authorization code and state from the callback URL
    let url = url::Url::parse(&callback_url)?;
    let mut code = None;
    let mut state = None;

    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "code" => code = Some(value.to_string()),
            "state" => state = Some(value.to_string()),
            _ => {}
        }
    }

    let code = code.ok_or("No authorization code found in callback URL")?;
    let state = state.ok_or("No state parameter found in callback URL")?;

    // Exchange the code for a token
    let token = oauth_client.exchange_code(code, state).await?;

    Ok(token)
}
