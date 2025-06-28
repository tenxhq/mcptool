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
        AuthCommands::Add { name } => add_command(name, output).await,
        AuthCommands::List => list_command(output).await,
        AuthCommands::Remove { name } => remove_command(name, output).await,
    }
}
