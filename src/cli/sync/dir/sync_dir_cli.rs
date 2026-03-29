use crate::cli::sync::dir::set::SyncDirSetArgs;
use crate::cli::sync::dir::show::SyncDirShowArgs;
use arbitrary::Arbitrary;
use facet::Facet;
use figue as args;

/// Sync-directory commands.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct SyncDirArgs {
    /// The sync-dir subcommand to run.
    #[facet(args::subcommand)]
    pub command: SyncDirCommand,
}

/// Sync-directory subcommands.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[repr(u8)]
pub enum SyncDirCommand {
    /// Set the persisted sync directory.
    Set(SyncDirSetArgs),
    /// Show the current sync directory.
    Show(SyncDirShowArgs),
}

impl SyncDirArgs {
    /// # Errors
    ///
    /// This function will return an error if the selected subcommand fails.
    pub async fn invoke(self) -> eyre::Result<()> {
        match self.command {
            SyncDirCommand::Set(args) => args.invoke().await,
            SyncDirCommand::Show(args) => args.invoke().await,
        }
    }
}