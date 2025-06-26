use std::sync::Arc;
use std::sync::Mutex;
use tenx_mcp::{
    Error, Result, Server, ServerConn, ServerCtx,
    schema::{
        ClientCapabilities, ClientNotification, Cursor, Implementation, InitializeResult,
        ListToolsResult, ServerCapabilities, Tool, ToolInputSchema,
    },
};

/// A test server connection that logs all interactions verbosely
#[derive(Clone)]
struct TestServerConn {
    request_counter: Arc<Mutex<u64>>,
}

impl TestServerConn {
    fn new() -> Self {
        Self {
            request_counter: Arc::new(Mutex::new(0)),
        }
    }

    fn log_request(&self, method: &str, params: &str) {
        let mut counter = self.request_counter.lock().unwrap();
        *counter += 1;
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        println!("\n[{timestamp}] REQUEST #{counter} - {method}");
        println!("Parameters: {params}");
    }

    fn log_response(&self, method: &str, response: &str) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        println!("[{timestamp}] RESPONSE - {method}");
        println!("Result: {response}");
    }

    fn log_notification(&self, notification: &str) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        println!("\n[{timestamp}] NOTIFICATION");
        println!("Content: {notification}");
    }
}

#[async_trait::async_trait]
impl ServerConn for TestServerConn {
    async fn on_connect(&self, _context: &ServerCtx) -> Result<()> {
        println!("\n=== TEST SERVER CONNECTED ===");
        println!("Client connected to test server");
        Ok(())
    }

    async fn on_disconnect(&self) -> Result<()> {
        println!("\n=== TEST SERVER DISCONNECTED ===");
        println!("Client disconnected from test server");
        Ok(())
    }

    async fn initialize(
        &self,
        _context: &ServerCtx,
        protocol_version: String,
        capabilities: ClientCapabilities,
        client_info: tenx_mcp::schema::Implementation,
    ) -> Result<InitializeResult> {
        let params = serde_json::json!({
            "protocol_version": protocol_version,
            "capabilities": capabilities,
            "client_info": client_info,
        });
        self.log_request(
            "initialize",
            &serde_json::to_string_pretty(&params).unwrap(),
        );

        let server_info = Implementation::new("mcptool-testserver", env!("CARGO_PKG_VERSION"));

        let result = InitializeResult {
            protocol_version: tenx_mcp::schema::LATEST_PROTOCOL_VERSION.to_string(),
            capabilities: ServerCapabilities::default()
                .with_tools(Some(true)),
            server_info,
            instructions: Some("This is a test server that logs all interactions. It only supports the 'echo' tool.".to_string()),
            meta: None,
        };

        let response = serde_json::to_string_pretty(&result).unwrap();
        self.log_response("initialize", &response);

        Ok(result)
    }

    async fn pong(&self, _context: &ServerCtx) -> Result<()> {
        self.log_request("ping", "{}");
        self.log_response("ping", "pong");
        Ok(())
    }

    async fn list_tools(
        &self,
        _context: &ServerCtx,
        cursor: Option<Cursor>,
    ) -> Result<ListToolsResult> {
        let params = serde_json::json!({
            "cursor": cursor,
        });
        self.log_request(
            "tools/list",
            &serde_json::to_string_pretty(&params).unwrap(),
        );

        let echo_tool = Tool::new(
            "echo",
            ToolInputSchema::default()
                .with_property(
                    "message",
                    serde_json::json!({
                        "type": "string",
                        "description": "The message to echo back"
                    }),
                )
                .with_required("message"),
        )
        .with_description("Echoes back the provided message");

        let result = ListToolsResult::default().with_tool(echo_tool);

        let response = serde_json::to_string_pretty(&result).unwrap();
        self.log_response("tools/list", &response);

        Ok(result)
    }

    async fn call_tool(
        &self,
        _context: &ServerCtx,
        name: String,
        arguments: Option<std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<tenx_mcp::schema::CallToolResult> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments,
        });
        self.log_request(
            "tools/call",
            &serde_json::to_string_pretty(&params).unwrap(),
        );

        if name != "echo" {
            return Err(Error::ToolNotFound(format!("Unknown tool: {name}")));
        }

        let message = arguments
            .as_ref()
            .and_then(|args| args.get("message"))
            .and_then(|v| v.as_str())
            .unwrap_or("No message provided");

        let result =
            tenx_mcp::schema::CallToolResult::new().with_text_content(format!("Echo: {message}"));

        let response = serde_json::to_string_pretty(&result).unwrap();
        self.log_response("tools/call", &response);

        Ok(result)
    }

    async fn notification(
        &self,
        _context: &ServerCtx,
        notification: ClientNotification,
    ) -> Result<()> {
        let notif_json = serde_json::to_string_pretty(&notification).unwrap();
        self.log_notification(&notif_json);
        Ok(())
    }
}

pub async fn run_test_server(stdio: bool, port: u16) -> Result<()> {
    println!("=== MCPTOOL TEST SERVER ===");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!("Protocol: {}", tenx_mcp::schema::LATEST_PROTOCOL_VERSION);

    let server = Server::default()
        .with_connection(TestServerConn::new)
        .with_capabilities(ServerCapabilities::default().with_tools(Some(true)));

    if stdio {
        println!("Transport: stdio");
        println!("Waiting for client connection on stdin/stdout...\n");
        server.serve_stdio().await?;
    } else {
        let addr = format!("127.0.0.1:{port}");
        println!("Transport: HTTP");
        println!("Listening on: http://{addr}");
        println!("Press Ctrl+C to stop the server\n");

        let handle = server.serve_http(&addr).await?;

        // Wait for Ctrl+C
        tokio::signal::ctrl_c().await.unwrap();
        println!("\nShutting down server...");
        handle.stop().await?;
    }

    Ok(())
}

