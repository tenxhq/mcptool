use crate::utils::TimedFuture;
use tenx_mcp::{Client, ServerAPI};

pub async fn ping(
    client: &mut Client<()>,
    output: &crate::output::Output,
) -> Result<(), Box<dyn std::error::Error>> {
    client.ping().timed("Pinged").await?;
    output.success("Ping successful!")?;
    Ok(())
}
