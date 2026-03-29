use crate::cli::import::postgres::ImportPostgresArgs;
use arbitrary::Arbitrary;
use facet::Facet;
use figue as args;

/// Import data from older storage backends.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct ImportArgs {
    /// The import subcommand to run.
    #[facet(args::subcommand)]
    pub command: ImportCommand,
}

/// Import subcommands.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[repr(u8)]
pub enum ImportCommand {
    /// Import from an older Postgres-backed workflow.
    Postgres(ImportPostgresArgs),
}

impl ImportArgs {
    /// # Errors
    ///
    /// This function will return an error if the selected subcommand fails.
    pub async fn invoke(self) -> eyre::Result<()> {
        match self.command {
            ImportCommand::Postgres(args) => args.invoke().await,
        }
    }
}
