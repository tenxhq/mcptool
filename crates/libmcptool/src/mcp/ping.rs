use crate::{utils::TimedFuture, Result};
use tenx_mcp::{Client, ServerAPI};

pub async fn ping(client: &mut Client<()>, output: &crate::output::Output) -> Result<()> {
    client.ping().timed("Pinged").await?;
    output.ping()?;
    Ok(())
}
