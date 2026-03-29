use crate::cli::sync::dir::SyncDirArgs;
use crate::cli::sync::now::SyncNowArgs;
use arbitrary::Arbitrary;
use facet::Facet;
use figue as args;

/// Build or inspect the persisted sync database.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct SyncArgs {
    /// The sync subcommand to run.
    #[facet(args::subcommand)]
    pub command: SyncCommand,
}

/// Sync subcommands.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[repr(u8)]
pub enum SyncCommand {
    /// Show or set the sync directory.
    Dir(SyncDirArgs),
    /// Build the sync database from an ingestion source.
    Now(SyncNowArgs),
}

impl SyncArgs {
    /// # Errors
    ///
    /// This function will return an error if the selected subcommand fails.
    pub async fn invoke(self) -> eyre::Result<()> {
        match self.command {
            SyncCommand::Dir(args) => args.invoke().await,
            SyncCommand::Now(args) => args.invoke().await,
        }
    }
}
