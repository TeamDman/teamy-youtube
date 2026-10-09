pub mod api;
pub mod cache;
pub mod facet_shape;
pub mod fetch;
pub mod global_args;
pub mod home;
pub mod output;
pub mod sync;
pub mod takeout;

use crate::cli::api::ApiArgs;
use crate::cli::cache::CacheArgs;
use crate::cli::fetch::FetchArgs;
use crate::cli::global_args::GlobalArgs;
use crate::cli::home::HomeArgs;
use crate::cli::output::CliOutput;
use crate::cli::sync::SyncArgs;
use crate::cli::takeout::TakeoutArgs;
use arbitrary::Arbitrary;
use eyre::Context;
use facet::Facet;
use figue::FigueBuiltins;
use figue::{self as args};

/// teamy-youtube command line interface.
///
/// Environment variables:
/// - `TEAMY_YOUTUBE_HOME` overrides the resolved application home directory.
/// - `TEAMY_YOUTUBE_CACHE_DIR` overrides the resolved cache directory.
/// - `RUST_LOG` provides a tracing filter when `--log-filter` is omitted.
#[derive(Facet, Arbitrary, Debug)]
pub struct Cli {
    /// Global arguments (`debug`, `log_filter`, `log_file`).
    #[facet(flatten)]
    pub global_args: GlobalArgs,

    /// Standard CLI options (help, version, completions).
    #[facet(flatten)]
    #[arbitrary(default)]
    pub builtins: FigueBuiltins,

    /// The command to run.
    #[facet(args::subcommand)]
    pub command: Command,
}

impl PartialEq for Cli {
    fn eq(&self, other: &Self) -> bool {
        // Ignore builtins in comparison since FigueBuiltins doesn't implement PartialEq
        self.global_args == other.global_args && self.command == other.command
    }
}

impl Cli {
    /// # Errors
    ///
    /// This function will return an error if the tokio runtime cannot be built or if the command fails.
    pub fn invoke(self) -> eyre::Result<CliOutput> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .wrap_err("Failed to build tokio runtime")?;
        runtime.block_on(async move { self.command.invoke().await })
    }
}

/// teamy-youtube commands.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[repr(u8)]
pub enum Command {
    /// Manage persisted external API configuration.
    Api(ApiArgs),
    /// Inspect or manage the local throwaway cache directory.
    Cache(CacheArgs),
    /// Fetch metadata from external sources into the filesystem database.
    Fetch(FetchArgs),
    /// Inspect or open the roaming home directory.
    Home(HomeArgs),
    /// Reconcile referenced videos with local filesystem-backed data.
    Sync(SyncArgs),
    /// Inspect or read Takeout ZIPs without extraction or import.
    Takeout(TakeoutArgs),
}

impl Command {
    /// Safe static dispatch label; excludes paths, queries and credentials.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Api(_) => "api",
            Self::Cache(_) => "cache",
            Self::Fetch(_) => "fetch",
            Self::Home(_) => "home",
            Self::Sync(_) => "sync",
            Self::Takeout(_) => "takeout",
        }
    }

    /// # Errors
    ///
    /// This function will return an error if the subcommand fails.
    #[tracing::instrument(name = "command", skip_all, fields(command = self.name()))]
    pub async fn invoke(self) -> eyre::Result<CliOutput> {
        match self {
            Command::Api(args) => args.invoke().await,
            Command::Cache(args) => args.invoke().await,
            Command::Fetch(args) => args.invoke().await,
            Command::Home(args) => args.invoke().await,
            Command::Sync(args) => args.invoke().await,
            Command::Takeout(args) => args.invoke(),
        }
    }
}
