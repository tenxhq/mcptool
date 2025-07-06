use std::time::Duration;

use libmcptool::{client, ctx::Ctx, target::Target};
use tenx_mcp::schema::ServerNotification;
use tenx_mcp::{
    Result as McpResult, Server, ServerAPI, ServerConn, ServerCtx,
    schema::{LoggingLevel, ServerCapabilities},
};

// Simple test server connection for integration tests
#[derive(Clone)]
struct SimpleTestServerConn;

#[async_trait::async_trait]
impl ServerConn for SimpleTestServerConn {
    async fn on_connect(&self, _context: &ServerCtx, _remote_addr: &str) -> McpResult<()> {
        Ok(())
    }

    async fn on_shutdown(&self) -> McpResult<()> {
        Ok(())
    }

    async fn initialize(
        &self,
        _context: &ServerCtx,
        _protocol_version: String,
        _capabilities: tenx_mcp::schema::ClientCapabilities,
        _client_info: tenx_mcp::schema::Implementation,
    ) -> McpResult<tenx_mcp::schema::InitializeResult> {
        Ok(tenx_mcp::schema::InitializeResult::new("test-server").with_version("1.0.0"))
    }

    async fn set_level(&self, context: &ServerCtx, level: LoggingLevel) -> McpResult<()> {
        // Send notification
        let notification = ServerNotification::LoggingMessage {
            level,
            logger: Some("test-notification".to_string()),
            data: serde_json::json!({ "message": "test-notification-message" }),
        };
        context.notify(notification);
        Ok(())
    }

    async fn list_tools(
        &self,
        _context: &ServerCtx,
        _cursor: Option<tenx_mcp::schema::Cursor>,
    ) -> McpResult<tenx_mcp::schema::ListToolsResult> {
        Ok(tenx_mcp::schema::ListToolsResult::default())
    }

    async fn call_tool(
        &self,
        _context: &ServerCtx,
        _name: String,
        _arguments: Option<tenx_mcp::Arguments>,
    ) -> McpResult<tenx_mcp::schema::CallToolResult> {
        Ok(tenx_mcp::schema::CallToolResult::new().with_text_content("test result"))
    }
}

#[tokio::test]
async fn test_set_level_command_notifications_via_tcp() -> Result<(), Box<dyn std::error::Error>> {
    // Create a temporary directory for the test
    let temp_dir = tempfile::tempdir()?;
    let config_path = temp_dir.path().to_path_buf();

    // Create context
    let ctx = Ctx::new(config_path.clone(), None, false, true, false, 80)?;

    // Start simple test server
    let server = Server::default()
        .with_connection(|| SimpleTestServerConn)
        .with_capabilities(ServerCapabilities::default().with_tools(Some(true)));

    let server_handle = tokio::spawn(async move {
        server.serve_tcp("127.0.0.1:8080").await.unwrap();
    });

    // Wait a bit for server to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect to the testserver via TCP
    let target = Target::parse("tcp://127.0.0.1:8080")?;
    let (mut client, _init_result) = client::get_client(&ctx, &target).await?;

    // Test set_level command - this should work and the server should handle it
    // The client should not hang given that a notification message is sent on each call
    client.set_level(LoggingLevel::Debug).await?;
    client.set_level(LoggingLevel::Info).await?;
    client.set_level(LoggingLevel::Warning).await?;
    client.set_level(LoggingLevel::Error).await?;

    // TODO - Add assertions to check that the notifications were received correctly

    // Clean up
    server_handle.abort();

    Ok(())
}
#[tokio::test]
#[ignore = "Requires HTTP + SSE for notifications"]
async fn test_set_level_command_notifications_via_http() -> Result<(), Box<dyn std::error::Error>> {
    // Create a temporary directory for the test
    let temp_dir = tempfile::tempdir()?;
    let config_path = temp_dir.path().to_path_buf();

    // Create context
    let ctx = Ctx::new(config_path.clone(), None, false, true, false, 80)?;

    // Start simple test server
    let server = Server::default()
        .with_connection(|| SimpleTestServerConn)
        .with_capabilities(ServerCapabilities::default().with_tools(Some(true)));

    let server_handle = tokio::spawn(async move {
        server.serve_http("127.0.0.1:8080").await.unwrap();
    });

    // Wait a bit for server to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect to the testserver via HTTP
    let target = Target::parse("http://127.0.0.1:8080")?;
    let (mut client, _init_result) = client::get_client(&ctx, &target).await?;

    // Test set_level command - this should work and the server should handle it
    // The client should not hang given that a notification message is sent on each call
    client.set_level(LoggingLevel::Debug).await?;
    client.set_level(LoggingLevel::Info).await?;
    client.set_level(LoggingLevel::Warning).await?;
    client.set_level(LoggingLevel::Error).await?;

    // TODO - Add assertions to check that the notifications were received correctly

    // Clean up
    server_handle.abort();

    Ok(())
}
