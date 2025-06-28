mod add;
mod list;
mod remove;

use crate::output::Output;
use clap::Subcommand;

pub use add::add_command;
pub use list::list_command;
pub use remove::remove_command;

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
}

pub async fn handle_auth_command(
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
        } => {
            add_command(
                name,
                server_url,
                auth_url,
                token_url,
                client_id,
                client_secret,
                output,
            )
            .await
        }
        AuthCommands::List => list_command(output).await,
        AuthCommands::Remove { name } => remove_command(name, output).await,
    }
}
