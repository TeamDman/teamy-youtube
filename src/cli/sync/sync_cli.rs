use crate::cli::sync::channel::SyncChannelArgs;
use crate::cli::sync::dir::SyncDirArgs;
use crate::cli::sync::run::fetch_videos::SyncRunFetchVideosArgs;
use crate::cli::sync::run::takeout::SyncRunTakeoutArgs;
use crate::cli::sync::run::thumbnails::SyncRunThumbnailsArgs;
use crate::cli::sync::status::SyncStatusArgs;
use arbitrary::Arbitrary;
use facet::Facet;
use figue as args;
use std::time::Duration;
use std::time::SystemTime;

const DEFAULT_TEAMY_MFT_MAX_AGE: Duration = Duration::from_secs(2 * 60 * 60);

/// Build or inspect the persisted sync database.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct SyncArgs {
    /// Fail before teamy-mft-backed stages if the indexed cache is older than this age. Defaults to 2 hours.
    #[facet(args::named)]
    pub bail_if_mft_older_than: Option<String>,

    /// The sync subcommand to run, or omitted to run all sync stages.
    #[facet(args::subcommand)]
    pub command: Option<SyncCommand>,
}

/// Sync subcommands.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[repr(u8)]
pub enum SyncCommand {
    /// Discover, enqueue, and download tracked channel videos.
    Channel(SyncChannelArgs),
    /// Show or set the sync directory.
    Dir(SyncDirArgs),
    /// Summarize remaining sync work.
    Status(SyncStatusArgs),
    // yt[sync.takeout.command]
    /// Build the sync database from Google Takeout exports.
    Takeout(SyncRunTakeoutArgs),
    // yt[sync.fetch-videos.command]
    /// Fetch missing video data for videos already referenced in the filesystem database.
    Videos(SyncRunFetchVideosArgs),
    // yt[sync.thumbnails.command]
    /// Download thumbnail assets for videos with fetched raw data.
    Thumbnails(SyncRunThumbnailsArgs),
}

impl SyncArgs {
    /// # Errors
    ///
    /// This function will return an error if the selected subcommand fails.
    pub async fn invoke(self) -> eyre::Result<()> {
        let mft_max_age = parse_optional_mft_age_argument(self.bail_if_mft_older_than.as_deref())?;
        match self.command {
            Some(SyncCommand::Channel(args)) => args.invoke(mft_max_age).await,
            Some(SyncCommand::Dir(args)) => args.invoke().await,
            Some(SyncCommand::Status(args)) => args.invoke(mft_max_age).await,
            Some(SyncCommand::Takeout(args)) => args.invoke_with_mft_max_age(mft_max_age).await,
            Some(SyncCommand::Videos(args)) => args.invoke().await,
            Some(SyncCommand::Thumbnails(args)) => args.invoke().await,
            None => run_all_sync_stages(mft_max_age).await,
        }
    }
}

// yt[sync.all.command]
async fn run_all_sync_stages(mft_max_age: Option<Duration>) -> eyre::Result<()> {
    println!("sync-stage=takeout");
    SyncRunTakeoutArgs {
        dry_run: false,
        input_dir: None,
    }
    .invoke_with_mft_max_age(mft_max_age)
    .await?;

    println!("sync-stage=videos");
    SyncRunFetchVideosArgs { fetch_limit: None }
        .invoke()
        .await?;

    println!("sync-stage=thumbnails");
    SyncRunThumbnailsArgs {
        limit: None,
        refresh_videos_newer_than: None,
        refresh_thumbnails_older_than: None,
    }
    .invoke()
    .await
}

pub(crate) fn assert_teamy_mft_query_cache_fresh(max_age: Option<Duration>) -> eyre::Result<()> {
    let Some(max_age) = max_age else {
        return Ok(());
    };

    teamy_mft::status::TeamyMftStatus::load_all_drives()?
        .assert_query_ready_not_older_than(max_age, SystemTime::now())
}

fn parse_optional_mft_age_argument(value: Option<&str>) -> eyre::Result<Option<Duration>> {
    value
        .map(|value| {
            humantime::parse_duration(value)
                .map_err(|error| eyre::eyre!("invalid value for --bail-if-mft-older-than: {error}"))
        })
        .transpose()
        .map(|value| value.or(Some(DEFAULT_TEAMY_MFT_MAX_AGE)))
}
