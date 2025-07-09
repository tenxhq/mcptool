use libmcptool::{ctx::Ctx, mcp, output::Output, target::Target};
use tempfile::TempDir;

/// Create a test context with a temporary config directory
fn create_test_ctx() -> (Ctx, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().to_path_buf();
    let ctx =
        Ctx::new(config_path, None, false, false, false, 80).expect("Failed to create context");
    (ctx, temp_dir)
}

#[tokio::test]
async fn test_mcp_init_with_test_server() {
    // Start the test server on a random port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to local address");
    let port = listener
        .local_addr()
        .expect("Failed to get local address")
        .port();
    drop(listener); // Release the port so test server can bind to it

    let (ctx, _temp_dir) = create_test_ctx();

    // Spawn the test server in the background
    let server_handle = tokio::spawn(async move {
        libmcptool::testserver::run_test_server(&ctx, false, false, port, false).await
    });

    // Give the server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Test with JSON output
    {
        let output = Output::new(false, 80).with_json(true);
        let target =
            Target::parse(&format!("http://127.0.0.1:{port}")).expect("Failed to parse target");

        let (_client, init_result) = libmcptool::client::connect_to_server(&target, ())
            .await
            .expect("Failed to connect to server");

        let result = mcp::init(&init_result, &output);
        assert!(result.is_ok(), "init should succeed with JSON output");

        // Verify basic fields
        assert_eq!(init_result.server_info.name, "mcptool-testserver");
        assert_eq!(init_result.server_info.version, "0.1.0");
        assert_eq!(
            init_result.protocol_version,
            tenx_mcp::schema::LATEST_PROTOCOL_VERSION
        );
    }

    // Test with text output
    {
        let output = Output::new(false, 80).with_json(false);
        let target =
            Target::parse(&format!("http://127.0.0.1:{port}")).expect("Failed to parse target");

        let (_client, init_result) = libmcptool::client::connect_to_server(&target, ())
            .await
            .expect("Failed to connect to server");

        let result = mcp::init(&init_result, &output);
        assert!(result.is_ok(), "init should succeed with text output");
    }

    // Abort the server
    server_handle.abort();
}

#[tokio::test]
async fn test_mcp_init_output_format() {
    // This test verifies the init function handles both output modes correctly
    // We use a mock InitializeResult to avoid needing a real server

    let init_result = tenx_mcp::schema::InitializeResult {
        protocol_version: "2025-06-18".to_string(),
        capabilities: tenx_mcp::schema::ServerCapabilities {
            tools: Some(tenx_mcp::schema::ToolsCapability {
                list_changed: Some(true),
            }),
            resources: Some(tenx_mcp::schema::ResourcesCapability {
                subscribe: Some(true),
                list_changed: Some(false),
            }),
            prompts: Some(tenx_mcp::schema::PromptsCapability {
                list_changed: Some(false),
            }),
            logging: Some(serde_json::Value::Object(serde_json::Map::new())),
            completions: Some(serde_json::Value::Object(serde_json::Map::new())),
            experimental: Some({
                let mut map = std::collections::HashMap::new();
                map.insert(
                    "custom_feature".to_string(),
                    serde_json::json!({
                        "enabled": true,
                        "version": "1.0"
                    }),
                );
                map
            }),
        },
        server_info: tenx_mcp::schema::Implementation {
            name: "Test Server".to_string(),
            version: "1.2.3".to_string(),
            title: Some("Test MCP Server".to_string()),
        },
        instructions: Some("Test instructions\nWith multiple lines".to_string()),
        _meta: None,
    };

    // Test JSON output
    {
        let output = Output::new(false, 80).with_json(true);
        let result = mcp::init(&init_result, &output);
        assert!(result.is_ok(), "init should succeed with JSON output");
    }

    // Test text output
    {
        let output = Output::new(false, 80).with_json(false);
        let result = mcp::init(&init_result, &output);
        assert!(result.is_ok(), "init should succeed with text output");
    }

    // Test with minimal server (no optional fields)
    let minimal_init_result = tenx_mcp::schema::InitializeResult {
        protocol_version: "2025-06-18".to_string(),
        capabilities: tenx_mcp::schema::ServerCapabilities::default(),
        server_info: tenx_mcp::schema::Implementation {
            name: "Minimal".to_string(),
            version: "0.1.0".to_string(),
            title: None,
        },
        instructions: None,
        _meta: None,
    };

    {
        let output = Output::new(false, 80).with_json(false);
        let result = mcp::init(&minimal_init_result, &output);
        assert!(result.is_ok(), "init should succeed with minimal server");
    }
}
