use crate::ctx::VERSION;
use crate::output::{Output, OutputLayer};
use crate::target::Target;
use tenx_mcp::{
    Client,
    schema::{ClientCapabilities, InitializeResult},
};
use tracing::Level;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

pub async fn connect_to_server(
    target: &Target,
) -> Result<(Client<()>, InitializeResult), Box<dyn std::error::Error>> {
    let mut client =
        Client::new("mcptool", VERSION).with_capabilities(ClientCapabilities::default());

    let init_result = match target {
        Target::Tcp { host, port } => {
            let addr = format!("{host}:{port}");
            client
                .connect_tcp(&addr)
                .await
                .map_err(|e| format!("Failed to connect to TCP address {addr}: {e}"))?
        }
        Target::Stdio { command, args } => {
            println!(
                "Connecting to MCP server via command: {} {}",
                command,
                args.join(" ")
            );

            let mut cmd = tokio::process::Command::new(command);
            cmd.args(args);

            let _child = client
                .connect_process(cmd)
                .await
                .map_err(|e| format!("Failed to spawn MCP server process: {e}"))?;

            // The new API handles initialization automatically
            client
                .init()
                .await
                .map_err(|e| format!("Failed to initialize MCP client: {e}"))?
        }
        Target::Http { host, port } => {
            let url = format!("http://{host}:{port}");
            client
                .connect_http(&url)
                .await
                .map_err(|e| format!("Failed to connect to HTTP endpoint {url}: {e}"))?
        }
        Target::Https { host, port } => {
            let url = format!("https://{host}:{port}");
            client
                .connect_http(&url)
                .await
                .map_err(|e| format!("Failed to connect to HTTPS endpoint {url}: {e}"))?
        }
    };

    Ok((client, init_result))
}

pub fn init_logging(level: Option<Level>) -> Output {
    let output = Output::new();

    let env_filter = match level {
        Some(Level::ERROR) => EnvFilter::try_new("error").unwrap_or_default(),
        Some(Level::WARN) => EnvFilter::try_new("warn").unwrap_or_default(),
        Some(Level::INFO) => EnvFilter::try_new("info").unwrap_or_default(),
        Some(Level::DEBUG) => EnvFilter::try_new("debug").unwrap_or_default(),
        Some(Level::TRACE) => EnvFilter::try_new("trace").unwrap_or_default(),
        None => EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::try_new("info").unwrap()),
    };

    let output_layer = OutputLayer::new(output.clone());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(output_layer)
        .init();

    output
}

pub fn create_output_with_logging(
    logs: Option<Option<String>>,
) -> Result<Output, Box<dyn std::error::Error>> {
    if let Some(log_level) = logs {
        let level = match log_level.as_deref() {
            Some("error") => Some(Level::ERROR),
            Some("warn") => Some(Level::WARN),
            Some("info") => Some(Level::INFO),
            Some("debug") => Some(Level::DEBUG),
            Some("trace") => Some(Level::TRACE),
            Some(other) => {
                return Err(format!("Invalid log level: {other}").into());
            }
            None => Some(Level::INFO), // Default to INFO if --logs is used without a level
        };
        Ok(init_logging(level))
    } else {
        Ok(Output::new())
    }
}
