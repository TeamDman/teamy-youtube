use crate::cli::fetch::video::FetchVideoArgs;
use arbitrary::Arbitrary;
use facet::Facet;
use figue as args;

/// Fetch metadata from external sources into the sync database.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct FetchArgs {
    /// The fetch subcommand to run.
    #[facet(args::subcommand)]
    pub command: FetchCommand,
}

/// Fetch subcommands.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[repr(u8)]
pub enum FetchCommand {
    /// Fetch metadata for a specific `YouTube` video.
    Video(FetchVideoArgs),
}

impl FetchArgs {
    /// # Errors
    ///
    /// This function will return an error if the selected subcommand fails.
    pub async fn invoke(self) -> eyre::Result<()> {
        match self.command {
            FetchCommand::Video(args) => args.invoke().await,
        }
    }
}
