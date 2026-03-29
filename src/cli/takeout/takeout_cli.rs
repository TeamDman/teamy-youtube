use crate::cli::takeout::import::TakeoutImportArgs;
use arbitrary::Arbitrary;
use facet::Facet;
use figue as args;

/// Ingest Google Takeout exports.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct TakeoutArgs {
    /// The takeout subcommand to run.
    #[facet(args::subcommand)]
    pub command: TakeoutCommand,
}

/// Takeout subcommands.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[repr(u8)]
pub enum TakeoutCommand {
    /// Import known Google Takeout files into the local store.
    Import(TakeoutImportArgs),
}

impl TakeoutArgs {
    /// # Errors
    ///
    /// This function will return an error if the selected subcommand fails.
    pub async fn invoke(self) -> eyre::Result<()> {
        match self.command {
            TakeoutCommand::Import(args) => args.invoke().await,
        }
    }
}
