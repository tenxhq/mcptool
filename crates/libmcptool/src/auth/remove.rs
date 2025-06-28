use crate::ctx::Ctx;
use rustyline::DefaultEditor;

pub async fn remove_command(ctx: &Ctx, name: String) -> Result<(), Box<dyn std::error::Error>> {
    let storage = ctx.storage()?;

    // Check if the entry exists
    if !storage.list_auth()?.contains(&name) {
        return Err(format!("Authentication entry '{name}' not found").into());
    }

    // Get the auth details for confirmation
    let auth = storage.get_auth(&name)?;

    // Confirm removal
    ctx.output
        .warn(format!("About to remove authentication entry '{name}'"))?;
    ctx.output.text(format!("  Server: {}", auth.server_url))?;
    ctx.output
        .text(format!("  Client ID: {}", auth.client_id))?;
    ctx.output.text("")?;

    let mut rl = DefaultEditor::new()?;
    let confirmation = rl.readline("Are you sure you want to remove this entry? (y/N): ")?;

    if confirmation.trim().to_lowercase() != "y" {
        ctx.output.text("Removal cancelled.")?;
        return Ok(());
    }

    // Remove the entry
    storage.remove_auth(&name)?;

    ctx.output.success(format!(
        "Authentication entry '{name}' removed successfully."
    ))?;

    Ok(())
}
