use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tenx_mcp::{
    Error, Result, Server, ServerConn, ServerCtx,
    schema::{
        ClientCapabilities, ClientNotification, Cursor, InitializeResult, ListPromptsResult,
        ListResourcesResult, ListToolsResult, LoggingLevel, Prompt, PromptArgument,
        ReadResourceResult, Resource, ServerCapabilities, ServerNotification, Tool, ToolSchema,
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
    request_counter: Arc<AtomicU64>,
    output: Output,
    log_level: Arc<Mutex<LoggingLevel>>,
}

impl TestServerConn {
    fn new(output: Output) -> Self {
        Self {
            request_counter: Arc::new(AtomicU64::new(0)),
            output,
            log_level: Arc::new(Mutex::new(LoggingLevel::Error)),
        }
    }

    async fn send_notification(
        &self,
        context: &ServerCtx,
        notification: ServerNotification,
    ) -> Result<()> {
        let _ = self.output.h1("sending notification");
        let _ = self.output.text(format!(
            "content: {}",
            serde_json::to_string_pretty(&notification).unwrap()
        ));

        context.notify(notification)
    }

    async fn send_log_message(
        &self,
        context: &ServerCtx,
        level: LoggingLevel,
        message: String,
    ) -> Result<()> {
        let current_level = *self.log_level.lock().unwrap();

        // Check if message should be sent based on current log level
        if self.should_log_message(&level, &current_level) {
            let notification = ServerNotification::LoggingMessage {
                level,
                logger: Some("testserver".to_string()),
                data: serde_json::json!({ "message": message }),
            };
            self.send_notification(context, notification).await
        } else {
            Ok(())
        }
    }

    fn should_log_message(
        &self,
        message_level: &LoggingLevel,
        current_level: &LoggingLevel,
    ) -> bool {
        message_level >= current_level
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
        let _ = self.output.h1("initialize");
        let params = serde_json::json!({
            "protocol_version": protocol_version,
            "capabilities": capabilities,
            "client_info": client_info,
        });
        let _ = self.output.text(format!(
            "parameters: {}",
            serde_json::to_string_pretty(&params).unwrap()
        ));

        let result = InitializeResult::new("mcptool-testserver")
            .with_version(env!("CARGO_PKG_VERSION"))
            .with_tools(true)
            .with_prompts(true)
            .with_resources(true, true)
            .with_instructions("mcptool test server");

        let _ = self.output.text(format!(
            "result: {}",
            serde_json::to_string_pretty(&result).unwrap()
        ));

        Ok(result)
    }

    async fn pong(&self, _context: &ServerCtx) -> Result<()> {
        let _ = self.output.h1("pong");
        let _ = self.output.text("parameters: {}");
        let _ = self.output.text("result: pong");
        Ok(())
    }

    async fn list_tools(
        &self,
        _context: &ServerCtx,
        cursor: Option<Cursor>,
    ) -> Result<ListToolsResult> {
        let _ = self.output.h1("list_tools");
        let params = serde_json::json!({
            "cursor": cursor,
        });
        let _ = self.output.text(format!(
            "parameters: {}",
            serde_json::to_string_pretty(&params).unwrap()
        ));

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

        let _ = self.output.text(format!(
            "result: {}",
            serde_json::to_string_pretty(&result).unwrap()
        ));

        Ok(result)
    }

    async fn call_tool(
        &self,
        context: &ServerCtx,
        name: String,
        arguments: Option<tenx_mcp::Arguments>,
    ) -> Result<tenx_mcp::schema::CallToolResult> {
        let _ = self.output.h1("call_tool");
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments,
        });
        let _ = self.output.text(format!(
            "parameters: {}",
            serde_json::to_string_pretty(&params).unwrap()
        ));

        // Send notification about tool call
        self.send_log_message(
            context,
            LoggingLevel::Debug,
            format!("Tool '{}' called with arguments: {:?}", name, arguments),
        )
        .await?;

        if name != "echo" {
            self.send_log_message(
                context,
                LoggingLevel::Error,
                format!("Unknown tool requested: {}", name),
            )
            .await?;
            return Err(Error::ToolNotFound(format!("Unknown tool: {name}")));
        }

        let message = arguments
            .as_ref()
            .and_then(|args| args.get_string("message"))
            .unwrap_or_else(|| "No message provided".to_string());

        let result =
            tenx_mcp::schema::CallToolResult::new().with_text_content(format!("Echo: {message}"));

        let _ = self.output.text(format!(
            "result: {}",
            serde_json::to_string_pretty(&result).unwrap()
        ));

        // Send notification about successful tool execution
        self.send_log_message(
            context,
            LoggingLevel::Info,
            format!("Successfully executed tool '{}'", name),
        )
        .await?;

        Ok(result)
    }

    async fn notification(
        &self,
        context: &ServerCtx,
        notification: ClientNotification,
    ) -> Result<()> {
        let _ = self.output.h1("notification");
        let _ = self.output.text(format!(
            "content: {}",
            serde_json::to_string_pretty(&notification).unwrap()
        ));

        // Send a demo notification back to the client
        self.send_log_message(
            context,
            LoggingLevel::Info,
            format!("Received client notification: {:?}", notification),
        )
        .await?;

        Ok(())
    }

    async fn set_level(&self, context: &ServerCtx, level: LoggingLevel) -> Result<()> {
        let _ = self.output.h1("set_level");
        let _ = self.output.text(format!(
            "level: {}",
            serde_json::to_string_pretty(&level).unwrap()
        ));

        // Update the log level
        *self.log_level.lock().unwrap() = level;

        // Acknowledge the level change
        self.send_log_message(
            context,
            LoggingLevel::Info,
            format!("Log level changed to: {:?}", level),
        )
        .await?;

        // Send some demo messages at different levels to demonstrate filtering
        self.send_log_message(
            context,
            LoggingLevel::Debug,
            "This is a debug message".to_string(),
        )
        .await?;

        self.send_log_message(
            context,
            LoggingLevel::Info,
            "This is an info message".to_string(),
        )
        .await?;

        self.send_log_message(
            context,
            LoggingLevel::Warning,
            "This is a warning message".to_string(),
        )
        .await?;

        self.send_log_message(
            context,
            LoggingLevel::Error,
            "This is an error message".to_string(),
        )
        .await?;

        Ok(())
    }

    async fn list_prompts(
        &self,
        _context: &ServerCtx,
        cursor: Option<Cursor>,
    ) -> Result<ListPromptsResult> {
        let _ = self.output.h1("list_prompts");
        let params = serde_json::json!({
            "cursor": cursor,
        });
        let _ = self.output.text(format!(
            "parameters: {}",
            serde_json::to_string_pretty(&params).unwrap()
        ));

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

        let _ = self.output.text(format!(
            "result: {}",
            serde_json::to_string_pretty(&result).unwrap()
        ));

        Ok(result)
    }

    async fn get_prompt(
        &self,
        _context: &ServerCtx,
        name: String,
        arguments: Option<tenx_mcp::Arguments>,
    ) -> Result<tenx_mcp::schema::GetPromptResult> {
        let _ = self.output.h1("get_prompt");
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments,
        });
        let _ = self.output.text(format!(
            "parameters: {}",
            serde_json::to_string_pretty(&params).unwrap()
        ));

        let result = match name.as_str() {
            "greeting" => {
                let name = arguments
                    .as_ref()
                    .and_then(|args| args.get_string("name"))
                    .unwrap_or_else(|| "World".to_string());
                let style = arguments
                    .as_ref()
                    .and_then(|args| args.get_string("style"))
                    .unwrap_or_else(|| "casual".to_string());

                let message = match style.as_str() {
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
                    .and_then(|args| args.get_string("language"))
                    .unwrap_or_else(|| "unknown".to_string());
                let code = arguments
                    .as_ref()
                    .and_then(|args| args.get_string("code"))
                    .unwrap_or_default();

                let review = format!(
                    "Please review the following {language} code:\n\n```{language}\n{code}\n```\n\nProvide feedback on code quality, potential bugs, and improvements."
                );

                tenx_mcp::schema::GetPromptResult::new()
                    .with_description("Code review request")
                    .with_message(tenx_mcp::schema::PromptMessage::user_text(review))
            }
            _ => return Err(Error::MethodNotFound(format!("Unknown prompt: {name}"))),
        };

        let _ = self.output.text(format!(
            "result: {}",
            serde_json::to_string_pretty(&result).unwrap()
        ));

        Ok(result)
    }

    async fn list_resources(
        &self,
        _context: &ServerCtx,
        cursor: Option<Cursor>,
    ) -> Result<ListResourcesResult> {
        let _ = self.output.h1("list_resources");
        let params = serde_json::json!({
            "cursor": cursor,
        });
        let _ = self.output.text(format!(
            "parameters: {}",
            serde_json::to_string_pretty(&params).unwrap()
        ));

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

        let _ = self.output.text(format!(
            "result: {}",
            serde_json::to_string_pretty(&result).unwrap()
        ));

        Ok(result)
    }

    async fn read_resource(&self, context: &ServerCtx, uri: String) -> Result<ReadResourceResult> {
        let _ = self.output.h1("read_resource");
        let params = serde_json::json!({
            "uri": uri,
        });
        let _ = self.output.text(format!(
            "parameters: {}",
            serde_json::to_string_pretty(&params).unwrap()
        ));

        // Send notification about resource access
        self.send_log_message(
            context,
            LoggingLevel::Debug,
            format!("Resource accessed: {}", uri),
        )
        .await?;

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
                let metrics = format!(
                    "Server Metrics\n==============\n\nTotal requests processed: {}\nUptime: N/A\nMemory usage: N/A\nActive connections: 1",
                    self.request_counter.load(Ordering::Relaxed)
                );
                ReadResourceResult::new().with_text(uri, metrics)
            }
            _ => return Err(Error::ResourceNotFound { uri }),
        };

        let _ = self.output.text(format!(
            "result: {}",
            serde_json::to_string_pretty(&result).unwrap()
        ));

        Ok(result)
    }

    async fn list_resource_templates(
        &self,
        _context: &ServerCtx,
        cursor: Option<Cursor>,
    ) -> Result<tenx_mcp::schema::ListResourceTemplatesResult> {
        let _ = self.output.h1("list_resource_templates");
        let params = serde_json::json!({
            "cursor": cursor,
        });
        let _ = self.output.text(format!(
            "parameters: {}",
            serde_json::to_string_pretty(&params).unwrap()
        ));

        // Create sample resource templates
        let user_template = tenx_mcp::schema::ResourceTemplate::new(
            "user-profile",
            "user://testserver/{user_id}/profile",
        )
        .with_title("User Profile")
        .with_description("Access user profile information by ID")
        .with_mime_type("application/json");

        let log_template =
            tenx_mcp::schema::ResourceTemplate::new("dated-log", "log://testserver/{date}/entries")
                .with_title("Daily Log Entries")
                .with_description("Server log entries for a specific date (YYYY-MM-DD)")
                .with_mime_type("text/plain")
                .with_annotations(
                    tenx_mcp::schema::Annotations::new()
                        .with_priority(0.8)
                        .with_audience(vec![tenx_mcp::schema::Role::Assistant]),
                );

        let config_template = tenx_mcp::schema::ResourceTemplate::new(
            "config-section",
            "config://testserver/{section}/{key}",
        )
        .with_title("Configuration Values")
        .with_description("Access configuration values by section and key")
        .with_mime_type("text/plain");

        let metrics_template = tenx_mcp::schema::ResourceTemplate::new(
            "metric-history",
            "metrics://testserver/{metric_name}/history?period={period}",
        )
        .with_title("Metric History")
        .with_description("Historical data for a specific metric (period: 1h, 1d, 1w)")
        .with_mime_type("application/json")
        .with_annotations(
            tenx_mcp::schema::Annotations::new()
                .with_priority(0.5)
                .with_last_modified(chrono::Local::now().to_rfc3339()),
        );

        let result = tenx_mcp::schema::ListResourceTemplatesResult::default()
            .with_resource_template(user_template)
            .with_resource_template(log_template)
            .with_resource_template(config_template)
            .with_resource_template(metrics_template);

        let _ = self.output.text(format!(
            "result: {}",
            serde_json::to_string_pretty(&result).unwrap()
        ));

        Ok(result)
    }
}

pub async fn run_test_server(ctx: &Ctx, stdio: bool, tcp: bool, port: u16) -> Result<()> {
    // Validate that only one transport is specified
    let transport_count = [stdio, tcp].iter().filter(|&&x| x).count();
    if transport_count > 1 {
        return Err(Error::InvalidConfiguration(
            "Only one transport can be specified: --stdio, --tcp, or HTTP (default)".to_string(),
        ));
    }

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
    } else if tcp {
        let addr = format!("127.0.0.1:{port}");
        let _ = output.text("Transport: TCP");
        let _ = output.trace_success(format!("Listening on: tcp://{addr}"));
        let _ = output.text("Press Ctrl+C to stop the server");

        server.serve_tcp(&addr).await?;
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
