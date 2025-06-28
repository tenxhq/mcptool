use std::time::SystemTime;

use crate::{ctx::Ctx, output::Output};

pub async fn list_command(ctx: &Ctx, output: Output) -> Result<(), Box<dyn std::error::Error>> {
    let storage = ctx.storage()?;
    let auths = storage.get_all_auth()?;

    if auths.is_empty() {
        output.text("No authentication entries found.")?;
        output.text("")?;
        output.text("Use 'mcptool auth add <name>' to add a new authentication entry.")?;
        return Ok(());
    }

    output.heading(format!("Authentication entries ({}):", auths.len()))?;
    output.text("")?;

    // Find the maximum lengths for formatting
    let max_name_len = auths.iter().map(|a| a.name.len()).max().unwrap_or(4).max(4);
    let max_server_len = auths
        .iter()
        .map(|a| a.server_url.len())
        .max()
        .unwrap_or(6)
        .max(6);
    let max_client_len = auths
        .iter()
        .map(|a| a.client_id.len())
        .max()
        .unwrap_or(9)
        .max(9);

    // Print header
    output.text(format!(
        "{:<width_name$}  {:<width_server$}  {:<width_client$}  {}",
        "Name",
        "Server",
        "Client ID",
        "Status",
        width_name = max_name_len,
        width_server = max_server_len,
        width_client = max_client_len,
    ))?;

    output.text(format!(
        "{:-<width_name$}  {:-<width_server$}  {:-<width_client$}  {:-<10}",
        "",
        "",
        "",
        "",
        width_name = max_name_len,
        width_server = max_server_len,
        width_client = max_client_len,
    ))?;

    // Print each entry
    for auth in auths {
        let status = match (auth.access_token.is_some(), auth.expires_at) {
            (false, _) => "No token".to_string(),
            (true, None) => "Valid".to_string(),
            (true, Some(expires)) => {
                if expires > SystemTime::now() {
                    let duration = expires
                        .duration_since(SystemTime::now())
                        .unwrap_or_default();
                    let hours = duration.as_secs() / 3600;
                    let minutes = (duration.as_secs() % 3600) / 60;

                    if hours > 0 {
                        format!("Valid ({hours}h {minutes}m)")
                    } else {
                        format!("Valid ({minutes}m)")
                    }
                } else {
                    "Expired".to_string()
                }
            }
        };

        output.text(format!(
            "{:<width_name$}  {:<width_server$}  {:<width_client$}  {}",
            auth.name,
            auth.server_url,
            auth.client_id,
            status,
            width_name = max_name_len,
            width_server = max_server_len,
            width_client = max_client_len,
        ))?;
    }

    Ok(())
}
