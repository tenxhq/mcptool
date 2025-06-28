use crate::core::MCPTool;
use crate::common::connect_to_server;
use crate::output::Output;
use crate::target::Target;
use crate::utils::TimedFuture;
use tenx_mcp::{Client, ServerAPI};

pub async fn ping_command(
    target: Target,
    auth: Option<String>,
    mcptool: &MCPTool,
    output: Output,
) -> Result<(), Box<dyn std::error::Error>> {
    output.text(format!("Pinging {target}..."))?;

    ping_once(&target, auth, mcptool, &output).await?;

    Ok(())
}

async fn ping_once(
    target: &Target,
    auth: Option<String>,
    mcptool: &MCPTool,
    output: &Output,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut client, init_result) = if let Some(auth_name) = auth {
        super::connect_with_auth(target, &auth_name, mcptool, output).await?
    } else {
        connect_to_server(target)
            .timed("Connected and initialized")
            .await?
    };

    output.text(format!(
        "Server info: {} v{}",
        init_result.server_info.name, init_result.server_info.version
    ))?;

    execute_ping(&mut client, output).await?;

    Ok(())
}

async fn execute_ping(
    client: &mut Client<()>,
    output: &Output,
) -> Result<(), Box<dyn std::error::Error>> {
    client.ping().timed("Pinged").await?;
    output.success("Ping successful!")?;
    Ok(())
}
