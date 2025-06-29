use crate::{utils::TimedFuture, Result};
use tenx_mcp::{Client, ServerAPI};

pub async fn ping(client: &mut Client<()>, output: &crate::output::Output) -> Result<()> {
    client.ping().timed("Pinged", output).await?;
    output.ping()?;
    Ok(())
}

pub async fn listtools(client: &mut Client<()>, output: &crate::output::Output) -> Result<()> {
    let tools_result = client
        .list_tools(None)
        .timed("Tools retrieved", output)
        .await?;
    output.list_tools_result(&tools_result)?;
    Ok(())
}
