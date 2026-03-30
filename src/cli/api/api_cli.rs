use crate::cli::api::key::ApiKeyArgs;
use arbitrary::Arbitrary;
use facet::Facet;
use figue as args;

/// Manage persisted API configuration.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct ApiArgs {
    /// The API subcommand to run.
    #[facet(args::subcommand)]
    pub command: ApiCommand,
}

/// API subcommands.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[repr(u8)]
pub enum ApiCommand {
    /// Manage the persisted `YouTube` API key.
    Key(ApiKeyArgs),
}

impl ApiArgs {
    /// # Errors
    ///
    /// This function will return an error if the selected subcommand fails.
    pub async fn invoke(self) -> eyre::Result<()> {
        match self.command {
            ApiCommand::Key(args) => args.invoke().await,
        }
    }
}
