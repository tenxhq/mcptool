use std::time::{Duration, SystemTime};

use oauth2::{
    AuthUrl, ClientId, ClientSecret, RefreshToken, RequestTokenError, TokenResponse, TokenUrl,
    basic::BasicClient,
};

use crate::{Error, Result, ctx::Ctx};

pub async fn renew_command(ctx: &Ctx, name: String) -> Result<()> {
    ctx.output
        .h1(format!("Renewing OAuth authentication: {name}"))?;

    let storage = ctx.storage()?;
    let mut auth = storage.get_auth(&name)?;

    // Check if we have a refresh token
    let refresh_token = auth.refresh_token.as_ref().ok_or(Error::Other(
        "No refresh token available for this authentication entry".to_string(),
    ))?;

    ctx.output.text("Current token status:")?;
    match &auth.expires_at {
        Some(expires_at) => {
            let now = SystemTime::now();
            if *expires_at > now {
                let remaining = expires_at.duration_since(now).unwrap_or(Duration::ZERO);
                let hours = remaining.as_secs() / 3600;
                let minutes = (remaining.as_secs() % 3600) / 60;
                ctx.output
                    .text(format!("  Token expires in {hours}h {minutes}m"))?;
            } else {
                ctx.output.text("  Token is expired")?;
            }
        }
        None => {
            ctx.output.text("  No expiration information available")?;
        }
    }

    ctx.output.text("")?;
    ctx.output.text("Refreshing token...")?;

    // Create OAuth client directly using oauth2 crate
    let mut client = BasicClient::new(ClientId::new(auth.client_id.clone()))
        .set_auth_uri(
            AuthUrl::new(auth.auth_url.clone())
                .map_err(|e| Error::Other(format!("Invalid auth URL: {e}")))?,
        )
        .set_token_uri(
            TokenUrl::new(auth.token_url.clone())
                .map_err(|e| Error::Other(format!("Invalid token URL: {e}")))?,
        );

    if let Some(client_secret) = auth.client_secret.as_ref() {
        client = client.set_client_secret(ClientSecret::new(client_secret.clone()));
    }

    // Exchange refresh token for new access token
    let refresh_token_obj = RefreshToken::new(refresh_token.clone());
    let token_result = client
        .exchange_refresh_token(&refresh_token_obj)
        .request_async(&reqwest::Client::new())
        .await
        .map_err(|e| {
            Error::Other(match e {
                RequestTokenError::ServerResponse(response) => {
                    format!("Server error: {:?}", response.error())
                }
                RequestTokenError::Request(e) => format!("Request error: {e}"),
                RequestTokenError::Parse(e, _) => format!("Parse error: {e}"),
                RequestTokenError::Other(e) => format!("Other error: {e}"),
            })
        })?;

    // Update the stored auth with new token information
    auth.access_token = Some(token_result.access_token().secret().clone());

    // Update refresh token if a new one was provided
    if let Some(new_refresh_token) = token_result.refresh_token() {
        auth.refresh_token = Some(new_refresh_token.secret().clone());
    }

    // Update expiration time based on the response
    auth.expires_at = token_result
        .expires_in()
        .map(|duration| SystemTime::now() + duration);

    // Save the updated auth
    storage.store_auth(&auth)?;

    ctx.output.trace_success("Token refreshed successfully!")?;
    ctx.output.text("")?;
    ctx.output.text("New token status:")?;

    if let Some(expires_at) = auth.expires_at {
        let now = SystemTime::now();
        if expires_at > now {
            let remaining = expires_at.duration_since(now).unwrap_or(Duration::ZERO);
            let hours = remaining.as_secs() / 3600;
            let minutes = (remaining.as_secs() % 3600) / 60;
            ctx.output
                .text(format!("  Token expires in {hours}h {minutes}m"))?;
        } else {
            ctx.output.text("  Token is already expired")?;
        }
    } else {
        ctx.output.text("  No expiration information available")?;
    }

    Ok(())
}
