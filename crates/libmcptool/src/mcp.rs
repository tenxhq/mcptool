use crate::{Result, output, utils::TimedFuture};
use tenx_mcp::{Client, ServerAPI, schema::InitializeResult};

pub async fn ping(client: &mut Client<()>, output: &crate::output::Output) -> Result<()> {
    output.text("Pinging")?;
    client.ping().timed("   response", output).await?;
    output.ping()?;
    Ok(())
}

pub async fn listtools(client: &mut Client<()>, output: &crate::output::Output) -> Result<()> {
    output.text("Listing tools")?;
    let tools_result = client
        .list_tools(None)
        .timed("    response", output)
        .await?;
    output::listtools::list_tools_result(output, &tools_result)?;
    Ok(())
}

pub fn init(init_result: &InitializeResult, output: &crate::output::Output) -> Result<()> {
    output::initresult::init_result(output, init_result)?;
    Ok(())
}

pub async fn listresources(client: &mut Client<()>, output: &crate::output::Output) -> Result<()> {
    output.text("Listing resources")?;
    let resources_result = client
        .list_resources(None)
        .timed("    response", output)
        .await?;
    output::listresources::list_resources_result(output, &resources_result)?;
    Ok(())
}

pub async fn listprompts(client: &mut Client<()>, output: &crate::output::Output) -> Result<()> {
    output.text("Listing prompts")?;
    let prompts_result = client
        .list_prompts(None)
        .timed("    response", output)
        .await?;
    output::listprompts::list_prompts_result(output, &prompts_result)?;
    Ok(())
}

pub async fn listresourcetemplates(
    client: &mut Client<()>,
    output: &crate::output::Output,
) -> Result<()> {
    output.text("Listing resource templates")?;
    let templates_result = client
        .list_resource_templates(None)
        .timed("    response", output)
        .await?;
    output::listresourcetemplates::list_resource_templates_result(output, &templates_result)?;
    Ok(())
}
