use crate::cli::api::key::set::ApiKeySetArgs;
use crate::cli::api::key::validate::ApiKeyValidateArgs;
use arbitrary::Arbitrary;
use facet::Facet;
use figue as args;

/// `YouTube` API key commands.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct ApiKeyArgs {
    /// The API-key subcommand to run.
    #[facet(args::subcommand)]
    pub command: ApiKeyCommand,
}

/// `YouTube` API key subcommands.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[repr(u8)]
pub enum ApiKeyCommand {
    /// Persist a `YouTube` API key under the application home directory.
    Set(ApiKeySetArgs),
    /// Validate that a configured `YouTube` API key is usable.
    Validate(ApiKeyValidateArgs),
}

impl ApiKeyArgs {
    /// # Errors
    ///
    /// This function will return an error if the selected subcommand fails.
    pub async fn invoke(self) -> eyre::Result<()> {
        match self.command {
            ApiKeyCommand::Set(args) => args.invoke().await,
            ApiKeyCommand::Validate(args) => args.invoke().await,
        }
    }
}
