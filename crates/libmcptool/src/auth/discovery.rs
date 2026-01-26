//! OAuth authorization server metadata discovery.
//!
//! This module provides functionality to discover OAuth authorization server metadata
//! from the `.well-known/oauth-authorization-server` endpoint as per RFC 8414 and the
//! MCP authorization specification.

use tmcp::auth::{AuthorizationDiscoveryClient, AuthorizationServerMetadata};
use url::Url;

use crate::{Error, Result, ctx::Ctx};

/// Extracts the authorization base URL from an MCP server URL.
///
/// Per MCP specification, the authorization base URL is determined by discarding
/// any path component from the MCP server URL.
///
/// For example:
/// - `https://mcp.linear.app` -> `https://mcp.linear.app`
/// - `https://api.example.com/v1/mcp` -> `https://api.example.com`
pub fn get_authorization_base_url(server_url: &str) -> Result<String> {
    let url =
        Url::parse(server_url).map_err(|e| Error::Format(format!("Invalid server URL: {e}")))?;

    let base = format!(
        "{}://{}{}",
        url.scheme(),
        url.host_str()
            .ok_or_else(|| Error::Format("Server URL must have a host".to_string()))?,
        url.port().map(|p| format!(":{p}")).unwrap_or_default()
    );

    Ok(base)
}

/// Discovers OAuth authorization server metadata from a server URL.
///
/// This function:
/// 1. Extracts the authorization base URL from the server URL
/// 2. Fetches the `.well-known/oauth-authorization-server` metadata document
/// 3. Returns the parsed metadata
pub async fn discover_metadata(server_url: &str) -> Result<AuthorizationServerMetadata> {
    let base_url = get_authorization_base_url(server_url)?;
    let client = AuthorizationDiscoveryClient::new();

    client
        .discover_authorization_server_metadata(&base_url)
        .await
        .map_err(|e| Error::Other(format!("Failed to discover OAuth metadata: {e}")))
}

/// Displays OAuth authorization server metadata to the output.
pub fn display_metadata(ctx: &Ctx, metadata: &AuthorizationServerMetadata) -> Result<()> {
    ctx.output.h1("OAuth Authorization Server Metadata")?;
    ctx.output.text("")?;

    if let Some(issuer) = &metadata.issuer {
        ctx.output.kv("Issuer", issuer)?;
    }

    if let Some(auth_endpoint) = &metadata.authorization_endpoint {
        ctx.output.kv("Authorization Endpoint", auth_endpoint)?;
    }

    if let Some(token_endpoint) = &metadata.token_endpoint {
        ctx.output.kv("Token Endpoint", token_endpoint)?;
    }

    if let Some(registration_endpoint) = &metadata.registration_endpoint {
        ctx.output
            .kv("Registration Endpoint", registration_endpoint)?;
        ctx.output
            .trace_success("Dynamic client registration is supported")?;
    } else {
        ctx.output
            .trace_warn("Dynamic client registration is NOT supported")?;
    }

    ctx.output.text("")?;

    if let Some(response_types) = &metadata.response_types_supported {
        ctx.output
            .kv("Response Types Supported", response_types.join(", "))?;
    }

    if let Some(grant_types) = &metadata.grant_types_supported {
        ctx.output
            .kv("Grant Types Supported", grant_types.join(", "))?;
    }

    if let Some(auth_methods) = &metadata.token_endpoint_auth_methods_supported {
        ctx.output
            .kv("Token Auth Methods", auth_methods.join(", "))?;
    }

    if let Some(scopes) = &metadata.scopes_supported {
        ctx.output.kv("Scopes Supported", scopes.join(", "))?;
    }

    Ok(())
}

/// Discovers metadata and runs the discovery command.
pub async fn discover_command(ctx: &Ctx, server_url: String) -> Result<()> {
    let base_url = get_authorization_base_url(&server_url)?;

    ctx.output.text(format!(
        "Discovering OAuth metadata from: {base_url}/.well-known/oauth-authorization-server"
    ))?;
    ctx.output.text("")?;

    let metadata = discover_metadata(&server_url).await?;
    display_metadata(ctx, &metadata)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_authorization_base_url() {
        // Simple URL without path
        assert_eq!(
            get_authorization_base_url("https://mcp.linear.app").unwrap(),
            "https://mcp.linear.app"
        );

        // URL with trailing slash
        assert_eq!(
            get_authorization_base_url("https://mcp.linear.app/").unwrap(),
            "https://mcp.linear.app"
        );

        // URL with path (should strip path per MCP spec)
        assert_eq!(
            get_authorization_base_url("https://api.example.com/v1/mcp").unwrap(),
            "https://api.example.com"
        );

        // URL with port
        assert_eq!(
            get_authorization_base_url("https://localhost:8080/mcp").unwrap(),
            "https://localhost:8080"
        );

        // HTTP URL
        assert_eq!(
            get_authorization_base_url("http://localhost:3000").unwrap(),
            "http://localhost:3000"
        );
    }

    #[test]
    fn test_get_authorization_base_url_invalid() {
        // Invalid URL
        assert!(get_authorization_base_url("not-a-url").is_err());

        // Missing scheme
        assert!(get_authorization_base_url("mcp.linear.app").is_err());
    }
}
