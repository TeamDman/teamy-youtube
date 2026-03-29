use crate::cli::sync::now::takeout::SyncNowTakeoutArgs;
use arbitrary::Arbitrary;
use facet::Facet;
use figue as args;

/// Build the sync database from a datasource.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct SyncNowArgs {
    /// The datasource to sync from.
    #[facet(args::subcommand)]
    pub command: SyncNowCommand,
}

/// Sync-now subcommands.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[repr(u8)]
pub enum SyncNowCommand {
    /// Build the sync database from Google Takeout exports.
    Takeout(SyncNowTakeoutArgs),
}

impl SyncNowArgs {
    /// # Errors
    ///
    /// This function will return an error if the selected subcommand fails.
    pub async fn invoke(self) -> eyre::Result<()> {
        match self.command {
            SyncNowCommand::Takeout(args) => args.invoke().await,
        }
    }
}
