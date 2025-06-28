use crate::utils::TimedFuture;
use tenx_mcp::{Client, ServerAPI};

pub async fn listtools(
    client: &mut Client<()>,
    output: &crate::output::Output,
) -> Result<(), Box<dyn std::error::Error>> {
    let tools_result = client.list_tools(None).timed("Tools retrieved").await?;
    output.list_tools_result(&tools_result)?;
    Ok(())
}
