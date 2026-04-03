use crate::cli::sync::dir::SyncDirArgs;
use crate::cli::sync::run::fetch_videos::SyncRunFetchVideosArgs;
use crate::cli::sync::run::takeout::SyncRunTakeoutArgs;
use crate::cli::sync::run::thumbnails::SyncRunThumbnailsArgs;
use arbitrary::Arbitrary;
use facet::Facet;
use figue as args;

/// Build or inspect the persisted sync database.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct SyncArgs {
    /// The sync subcommand to run, or omitted to run all sync stages.
    #[facet(args::subcommand)]
    pub command: Option<SyncCommand>,
}

/// Sync subcommands.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[repr(u8)]
pub enum SyncCommand {
    /// Show or set the sync directory.
    Dir(SyncDirArgs),
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
        match self.command {
            Some(SyncCommand::Dir(args)) => args.invoke().await,
            Some(SyncCommand::Takeout(args)) => args.invoke().await,
            Some(SyncCommand::Videos(args)) => args.invoke().await,
            Some(SyncCommand::Thumbnails(args)) => args.invoke().await,
            None => run_all_sync_stages().await,
        }
    }
}

// yt[sync.all.command]
async fn run_all_sync_stages() -> eyre::Result<()> {
    println!("sync-stage=takeout");
    SyncRunTakeoutArgs {
        dry_run: false,
        input_dir: None,
    }
    .invoke()
    .await?;

    println!("sync-stage=videos");
    SyncRunFetchVideosArgs { fetch_limit: None }
        .invoke()
        .await?;

    println!("sync-stage=thumbnails");
    SyncRunThumbnailsArgs {
        refresh_videos_newer_than: None,
        refresh_thumbnails_older_than: None,
    }
    .invoke()
    .await
}
