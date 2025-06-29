use crate::{output, utils::TimedFuture, Result};
use tenx_mcp::{Client, ServerAPI};

pub async fn ping(client: &mut Client<()>, output: &crate::output::Output) -> Result<()> {
    client.ping().timed("Response", output).await?;
    output.ping()?;
    Ok(())
}

pub async fn listtools(client: &mut Client<()>, output: &crate::output::Output) -> Result<()> {
    let tools_result = client.list_tools(None).timed("Response", output).await?;
    output::listtools::list_tools_result(output, &tools_result)?;
    Ok(())
}
