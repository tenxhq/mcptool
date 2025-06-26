use crate::VERSION;
use crate::target::Target;
use tenx_mcp::{
    Client,
    schema::{ClientCapabilities, InitializeResult},
};

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
