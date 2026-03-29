use crate::cli::analyze::watch_later::AnalyzeWatchLaterArgs;
use arbitrary::Arbitrary;
use facet::Facet;
use figue as args;

/// Produce reports from local metadata and event data.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct AnalyzeArgs {
    /// The analyze subcommand to run.
    #[facet(args::subcommand)]
    pub command: AnalyzeCommand,
}

/// Analyze subcommands.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[repr(u8)]
pub enum AnalyzeCommand {
    /// Analyze the Watch Later playlist.
    WatchLater(AnalyzeWatchLaterArgs),
}

impl AnalyzeArgs {
    /// # Errors
    ///
    /// This function will return an error if the selected subcommand fails.
    pub async fn invoke(self) -> eyre::Result<()> {
        match self.command {
            AnalyzeCommand::WatchLater(args) => args.invoke().await,
        }
    }
}
