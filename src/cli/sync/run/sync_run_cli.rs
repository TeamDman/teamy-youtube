use crate::cli::sync::run::takeout::SyncRunTakeoutArgs;
use arbitrary::Arbitrary;
use facet::Facet;
use figue as args;

/// Build the sync database from a datasource.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct SyncRunArgs {
    /// The datasource to sync from.
    #[facet(args::subcommand)]
    pub command: SyncRunCommand,
}

/// Sync-run subcommands.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[repr(u8)]
pub enum SyncRunCommand {
    /// Build the sync database from Google Takeout exports.
    Takeout(SyncRunTakeoutArgs),
}

impl SyncRunArgs {
    /// # Errors
    ///
    /// This function will return an error if the selected subcommand fails.
    pub async fn invoke(self) -> eyre::Result<()> {
        match self.command {
            SyncRunCommand::Takeout(args) => args.invoke().await,
        }
    }
}
