mod add;
mod list;
mod remove;
mod renew;

use crate::ctx::Ctx;
use crate::output::Output;
use clap::Subcommand;

pub use add::{AddCommandArgs, add_command};
pub use list::list_command;
pub use remove::remove_command;
pub use renew::renew_command;

#[derive(Subcommand)]
pub enum AuthCommands {
    /// Add a new OAuth authentication entry
    Add {
        /// Name for the authentication entry
        name: String,

        /// Server URL (e.g., https://api.example.com)
        #[arg(long)]
        server_url: Option<String>,

        /// OAuth authorization URL
        #[arg(long)]
        auth_url: Option<String>,

        /// OAuth token URL
        #[arg(long)]
        token_url: Option<String>,

        /// OAuth client ID
        #[arg(long)]
        client_id: Option<String>,

        /// OAuth client secret
        #[arg(long)]
        client_secret: Option<String>,

        /// OAuth redirect URL (if not provided, will use local server)
        #[arg(long)]
        redirect_url: Option<String>,

        /// Resource/Audience parameter for OAuth
        #[arg(long)]
        resource: Option<String>,

        /// OAuth scopes (comma-separated)
        #[arg(long)]
        scopes: Option<String>,

        /// Show the redirect URL that will be used without starting OAuth flow
        #[arg(long)]
        show_redirect_url: bool,
    },

    /// List all stored authentication entries
    #[command(alias = "ls")]
    List,

    /// Remove an authentication entry
    #[command(alias = "rm")]
    Remove {
        /// Name of the authentication entry to remove
        name: String,
    },

    /// Renew the access token for an authentication entry using the refresh token
    Renew {
        /// Name of the authentication entry to renew
        name: String,
    },
}

pub async fn handle_auth_command(
    ctx: &Ctx,
    command: AuthCommands,
    output: Output,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        AuthCommands::Add {
            name,
            server_url,
            auth_url,
            token_url,
            client_id,
            client_secret,
            redirect_url,
            resource,
            scopes,
            show_redirect_url,
        } => {
            let args = AddCommandArgs {
                name,
                server_url,
                auth_url,
                token_url,
                client_id,
                client_secret,
                redirect_url,
                resource,
                scopes,
                show_redirect_url,
            };
            add_command(ctx, args, output).await
        }
        AuthCommands::List => list_command(ctx, output).await,
        AuthCommands::Remove { name } => remove_command(ctx, name, output).await,
        AuthCommands::Renew { name } => renew_command(ctx, name, output).await,
    }
}
