use crate::cli::config::init::ConfigInitArgs;
use crate::cli::config::show::ConfigShowArgs;
use arbitrary::Arbitrary;
use facet::Facet;
use figue as args;

/// Inspect or initialize repository configuration.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct ConfigArgs {
    /// The config subcommand to run.
    #[facet(args::subcommand)]
    pub command: ConfigCommand,
}

/// Config subcommands.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[repr(u8)]
pub enum ConfigCommand {
    /// Initialize the local home directory.
    Init(ConfigInitArgs),
    /// Show the resolved configuration values.
    Show(ConfigShowArgs),
}

impl ConfigArgs {
    /// # Errors
    ///
    /// This function will return an error if the selected subcommand fails.
    pub async fn invoke(self) -> eyre::Result<()> {
        match self.command {
            ConfigCommand::Init(args) => args.invoke().await,
            ConfigCommand::Show(args) => args.invoke().await,
        }
    }
}
