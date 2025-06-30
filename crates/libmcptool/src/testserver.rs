use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tenx_mcp::{
    Error, Result, Server, ServerConn, ServerCtx,
    schema::{
        ClientCapabilities, ClientNotification, Cursor, InitializeResult, ListPromptsResult,
        ListResourcesResult, ListToolsResult, Prompt, PromptArgument, ReadResourceResult, Resource,
        ServerCapabilities, Tool, ToolSchema,
    },
};

use crate::{ctx::Ctx, output::Output};

/// Sample user data structure for demonstrating JSON resource serving
#[derive(Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
    role: String,
    last_login: String,
}

/// Response structure for the users resource
#[derive(Serialize, Deserialize)]
struct UsersResponse {
    users: Vec<User>,
    total_count: usize,
    generated_at: String,
}

/// A test server connection that logs all interactions verbosely
#[derive(Clone)]
struct TestServerConn {
    request_counter: Arc<Mutex<u64>>,
    output: Output,
}

impl TestServerConn {
    fn new(output: Output) -> Self {
        Self {
            request_counter: Arc::new(Mutex::new(0)),
            output,
        }
    }

    fn log_request(&self, method: &str, params: &str) {
        let mut counter = self.request_counter.lock().unwrap();
        *counter += 1;
        let _ = self.output.h1(format!("request #{counter} - {method}"));
        let _ = self.output.text(format!("parameters: {params}"));
    }

    fn log_response(&self, method: &str, response: &str) {
        let _ = self.output.h1(format!("response - {method}"));
        let _ = self.output.text(format!("result: {response}"));
    }

    fn log_notification(&self, notification: &str) {
        let _ = self.output.h1("notification");
        let _ = self.output.text(format!("content: {notification}"));
    }
}

#[async_trait::async_trait]
impl ServerConn for TestServerConn {
    async fn on_connect(&self, _context: &ServerCtx, remote_addr: &str) -> Result<()> {
        let _ = self
            .output
            .trace_success(format!("client connected from {remote_addr}"));
        Ok(())
    }

    async fn on_shutdown(&self) -> Result<()> {
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

        let result = InitializeResult::new("mcptool-testserver")
            .with_version(env!("CARGO_PKG_VERSION"))
            .with_tools(true)
            .with_prompts(true)
            .with_resources(true, true)
            .with_instructions("mcptool test server");

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
            ToolSchema::default()
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

    async fn list_prompts(
        &self,
        _context: &ServerCtx,
        cursor: Option<Cursor>,
    ) -> Result<ListPromptsResult> {
        let params = serde_json::json!({
            "cursor": cursor,
        });
        self.log_request(
            "prompts/list",
            &serde_json::to_string_pretty(&params).unwrap(),
        );

        let greeting_prompt = Prompt {
            name: "greeting".to_string(),
            title: None,
            description: Some("Generate a greeting message".to_string()),
            arguments: Some(vec![
                PromptArgument {
                    name: "name".to_string(),
                    title: None,
                    description: Some("The name to greet".to_string()),
                    required: Some(true),
                },
                PromptArgument {
                    name: "style".to_string(),
                    title: None,
                    description: Some("The greeting style (formal/casual)".to_string()),
                    required: Some(false),
                },
            ]),
            _meta: None,
        };

        let code_review_prompt = Prompt {
            name: "code_review".to_string(),
            title: None,
            description: Some("Review code and provide feedback".to_string()),
            arguments: Some(vec![
                PromptArgument {
                    name: "language".to_string(),
                    title: None,
                    description: Some("Programming language of the code".to_string()),
                    required: Some(true),
                },
                PromptArgument {
                    name: "code".to_string(),
                    title: None,
                    description: Some("The code to review".to_string()),
                    required: Some(true),
                },
            ]),
            _meta: None,
        };

        let result = ListPromptsResult::default()
            .with_prompt(greeting_prompt)
            .with_prompt(code_review_prompt);

        let response = serde_json::to_string_pretty(&result).unwrap();
        self.log_response("prompts/list", &response);

        Ok(result)
    }

    async fn get_prompt(
        &self,
        _context: &ServerCtx,
        name: String,
        arguments: Option<std::collections::HashMap<String, String>>,
    ) -> Result<tenx_mcp::schema::GetPromptResult> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments,
        });
        self.log_request(
            "prompts/get",
            &serde_json::to_string_pretty(&params).unwrap(),
        );

        let result = match name.as_str() {
            "greeting" => {
                let name = arguments
                    .as_ref()
                    .and_then(|args| args.get("name"))
                    .map(|s| s.as_str())
                    .unwrap_or("World");
                let style = arguments
                    .as_ref()
                    .and_then(|args| args.get("style"))
                    .map(|s| s.as_str())
                    .unwrap_or("casual");

                let message = match style {
                    "formal" => format!("Good day, {name}. How may I assist you today?"),
                    _ => format!("Hey {name}! What's up?"),
                };

                tenx_mcp::schema::GetPromptResult::new()
                    .with_description("A personalized greeting")
                    .with_message(tenx_mcp::schema::PromptMessage::user_text(message))
            }
            "code_review" => {
                let language = arguments
                    .as_ref()
                    .and_then(|args| args.get("language"))
                    .map(|s| s.as_str())
                    .unwrap_or("unknown");
                let code = arguments
                    .as_ref()
                    .and_then(|args| args.get("code"))
                    .map(|s| s.as_str())
                    .unwrap_or("");

                let review = format!(
                    "Please review the following {language} code:\n\n```{language}\n{code}\n```\n\nProvide feedback on code quality, potential bugs, and improvements."
                );

                tenx_mcp::schema::GetPromptResult::new()
                    .with_description("Code review request")
                    .with_message(tenx_mcp::schema::PromptMessage::user_text(review))
            }
            _ => return Err(Error::MethodNotFound(format!("Unknown prompt: {name}"))),
        };

        let response = serde_json::to_string_pretty(&result).unwrap();
        self.log_response("prompts/get", &response);

        Ok(result)
    }

    async fn list_resources(
        &self,
        _context: &ServerCtx,
        cursor: Option<Cursor>,
    ) -> Result<ListResourcesResult> {
        let params = serde_json::json!({
            "cursor": cursor,
        });
        self.log_request(
            "resources/list",
            &serde_json::to_string_pretty(&params).unwrap(),
        );

        let log_resource = Resource::new("server-log", "log://testserver/current")
            .with_description("Current test server log")
            .with_mime_type("text/plain");

        let sample_data_resource = Resource::new("sample-data", "data://testserver/users.json")
            .with_description("Sample user data for testing")
            .with_mime_type("application/json")
            .with_size(512);

        let metrics_resource = Resource::new("server-metrics", "metrics://testserver/stats")
            .with_description("Server performance metrics")
            .with_mime_type("text/plain");

        let result = ListResourcesResult::default()
            .with_resource(log_resource)
            .with_resource(sample_data_resource)
            .with_resource(metrics_resource);

        let response = serde_json::to_string_pretty(&result).unwrap();
        self.log_response("resources/list", &response);

        Ok(result)
    }

    async fn read_resource(&self, _context: &ServerCtx, uri: String) -> Result<ReadResourceResult> {
        let params = serde_json::json!({
            "uri": uri,
        });
        self.log_request(
            "resources/read",
            &serde_json::to_string_pretty(&params).unwrap(),
        );

        let result = match uri.as_str() {
            "log://testserver/current" => {
                let log_content = format!(
                    "Test Server Log\n===============\n\n{} - Server started\n{} - Listening for connections\n{} - Processing requests...",
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
                );
                ReadResourceResult::new().with_text(uri, log_content)
            }
            "data://testserver/users.json" => {
                // Sample user data that demonstrates JSON resource serving
                let users = vec![
                    User {
                        id: 1,
                        name: "Alice Johnson".to_string(),
                        email: "alice@example.com".to_string(),
                        role: "admin".to_string(),
                        last_login: "2024-01-15T10:30:00Z".to_string(),
                    },
                    User {
                        id: 2,
                        name: "Bob Smith".to_string(),
                        email: "bob@example.com".to_string(),
                        role: "user".to_string(),
                        last_login: "2024-01-14T15:45:00Z".to_string(),
                    },
                    User {
                        id: 3,
                        name: "Charlie Davis".to_string(),
                        email: "charlie@example.com".to_string(),
                        role: "moderator".to_string(),
                        last_login: "2024-01-13T09:00:00Z".to_string(),
                    },
                ];

                let response = UsersResponse {
                    total_count: users.len(),
                    users,
                    generated_at: chrono::Local::now().to_rfc3339(),
                };

                ReadResourceResult::new().with_json(uri, &response).unwrap()
            }
            "metrics://testserver/stats" => {
                let counter = self.request_counter.lock().unwrap();
                let metrics = format!(
                    "Server Metrics\n==============\n\nTotal requests processed: {}\nUptime: N/A\nMemory usage: N/A\nActive connections: 1",
                    *counter
                );
                ReadResourceResult::new().with_text(uri, metrics)
            }
            _ => return Err(Error::ResourceNotFound { uri }),
        };

        let response = serde_json::to_string_pretty(&result).unwrap();
        self.log_response("resources/read", &response);

        Ok(result)
    }
}

pub async fn run_test_server(ctx: &Ctx, stdio: bool, port: u16) -> Result<()> {
    let output = if stdio {
        // In stdio mode, silence all output
        ctx.output.clone().with_quiet(true)
    } else {
        ctx.output.clone()
    };

    let _ = output.h1("mcptool testserver");
    let _ = output.text(format!("Version: {}", env!("CARGO_PKG_VERSION")));
    let _ = output.text(format!(
        "Protocol: {}",
        tenx_mcp::schema::LATEST_PROTOCOL_VERSION
    ));

    let output_for_conn = output.clone();
    let server = Server::default()
        .with_connection(move || TestServerConn::new(output_for_conn.clone()))
        .with_capabilities(
            ServerCapabilities::default()
                .with_tools(Some(true))
                .with_prompts(None)
                .with_resources(None, None),
        );

    if stdio {
        server.serve_stdio().await?;
    } else {
        let addr = format!("127.0.0.1:{port}");
        let _ = output.text("Transport: HTTP");
        let _ = output.trace_success(format!("Listening on: http://{addr}"));
        let _ = output.text("Press Ctrl+C to stop the server");

        let handle = server.serve_http(&addr).await?;

        // Wait for Ctrl+C
        tokio::signal::ctrl_c().await.unwrap();
        let _ = output.trace_warn("Shutting down server...");
        handle.stop().await?;
    }

    Ok(())
}
