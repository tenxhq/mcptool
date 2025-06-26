use crate::output::Output;
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
    output: Arc<Mutex<Output>>,
}

impl TestServerConn {
    fn new() -> Self {
        Self {
            request_counter: Arc::new(Mutex::new(0)),
            output: Arc::new(Mutex::new(Output::new())),
        }
    }

    fn log_request(&self, method: &str, params: &str) {
        let mut counter = self.request_counter.lock().unwrap();
        *counter += 1;
        let mut output = self.output.lock().unwrap();
        let _ = output.heading(&format!("request #{counter} - {method}"));
        let _ = output.text(&format!("parameters: {params}"));
    }

    fn log_response(&self, method: &str, response: &str) {
        let mut output = self.output.lock().unwrap();
        let _ = output.heading(&format!("response - {method}"));
        let _ = output.text(&format!("result: {response}"));
    }

    fn log_notification(&self, notification: &str) {
        let mut output = self.output.lock().unwrap();
        let _ = output.heading("notification");
        let _ = output.text(&format!("content: {notification}"));
    }
}

#[async_trait::async_trait]
impl ServerConn for TestServerConn {
    async fn on_connect(&self, _context: &ServerCtx) -> Result<()> {
        let mut output = self.output.lock().unwrap();
        let _ = output.success("client connected");
        Ok(())
    }

    async fn on_disconnect(&self) -> Result<()> {
        let mut output = self.output.lock().unwrap();
        let _ = output.warn("disconnected");
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
    let mut output = Output::new();
    let _ = output.heading("mcptool testserver");
    let _ = output.text(&format!("Version: {}", env!("CARGO_PKG_VERSION")));
    let _ = output.text(&format!(
        "Protocol: {}",
        tenx_mcp::schema::LATEST_PROTOCOL_VERSION
    ));

    let server = Server::default()
        .with_connection(TestServerConn::new)
        .with_capabilities(ServerCapabilities::default().with_tools(Some(true)));

    if stdio {
        let _ = output.text("Transport: stdio");
        let _ = output.text("Waiting for client connection on stdin/stdout...");
        server.serve_stdio().await?;
    } else {
        let addr = format!("127.0.0.1:{port}");
        let _ = output.text("Transport: HTTP");
        let _ = output.success(&format!("Listening on: http://{addr}"));
        let _ = output.text("Press Ctrl+C to stop the server");

        let handle = server.serve_http(&addr).await?;

        // Wait for Ctrl+C
        tokio::signal::ctrl_c().await.unwrap();
        let _ = output.warn("Shutting down server...");
        handle.stop().await?;
    }

    Ok(())
}
