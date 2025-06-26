use crate::target::Target;
use crate::VERSION;
use tenx_mcp::{
    Client,
    schema::{ClientCapabilities, InitializeResult},
};

pub async fn connect_to_server(
    target: &Target,
) -> Result<(Client<()>, InitializeResult), Box<dyn std::error::Error>> {
    let mut client = Client::new("mcptool", VERSION)
        .with_capabilities(ClientCapabilities::default());

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
        Target::Http { .. } | Target::Https { .. } => {
            return Err("HTTP/HTTPS connections are not yet supported".into());
        }
    };

    Ok((client, init_result))
}