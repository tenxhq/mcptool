use crate::common::connect_to_server;
use crate::target::Target;
use crate::utils::TimedFuture;
use tenx_mcp::{Client, ServerAPI};

pub async fn ping_command(target: Target) -> Result<(), Box<dyn std::error::Error>> {
    println!("Pinging {target}...");

    ping_once(&target).await?;

    Ok(())
}

async fn ping_once(target: &Target) -> Result<(), Box<dyn std::error::Error>> {
    let (mut client, init_result) = connect_to_server(target)
        .timed("Connected and initialized")
        .await?;

    println!(
        "Server info: {} v{}",
        init_result.server_info.name, init_result.server_info.version
    );

    execute_ping(&mut client).await?;

    Ok(())
}

async fn execute_ping(client: &mut Client<()>) -> Result<(), Box<dyn std::error::Error>> {
    client.ping().timed("Pinged").await?;
    Ok(())
}

