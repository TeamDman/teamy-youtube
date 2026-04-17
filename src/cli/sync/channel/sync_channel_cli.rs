use crate::cli::sync::channel::add::SyncChannelAddArgs;
use arbitrary::Arbitrary;
use facet::Facet;
use figue as args;
use std::time::Duration;

/// Discover, enqueue, and download tracked channel videos.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct SyncChannelArgs {
    /// Preview the channel sync plan without writing discovery or request events and without downloading anything.
    #[facet(args::named)]
    pub dry_run: bool,

    /// The channel subcommand to run.
    #[facet(args::subcommand)]
    pub command: Option<SyncChannelCommand>,
}

/// Channel sync subcommands.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[repr(u8)]
pub enum SyncChannelCommand {
    /// Add or update a tracked channel target.
    Add(SyncChannelAddArgs),
}

impl SyncChannelArgs {
    /// # Errors
    ///
    /// This function will return an error if discovery, queuing, teamy-mft checks,
    /// or video downloads fail.
    pub async fn invoke(self, mft_max_age: Option<Duration>) -> eyre::Result<()> {
        let Self { dry_run, command } = self;
        match command {
            Some(SyncChannelCommand::Add(args)) => {
                if dry_run {
                    eyre::bail!(
                        "--dry-run is only supported for `sync channel` without a subcommand"
                    );
                }
                args.invoke().await
            }
            None => run_channel_sync(mft_max_age, dry_run).await,
        }
    }
}

async fn run_channel_sync(mft_max_age: Option<Duration>, dry_run: bool) -> eyre::Result<()> {
    crate::cli::sync::assert_teamy_mft_query_cache_fresh(mft_max_age)?;
    let sync_dir = crate::paths::try_get_sync_dir()?;
    let plan = crate::channel_sync::plan_channel_sync(
        &sync_dir,
        if dry_run {
            crate::channel_sync::ChannelSyncPlanMode::StatusOnly
        } else {
            crate::channel_sync::ChannelSyncPlanMode::EnqueueMissingRequests
        },
    )
    .await?;

    if dry_run {
        print_channel_sync_summary(&sync_dir, &plan.summary, 0, 0, true);
        return Ok(());
    }

    let run_summary = crate::channel_sync::execute_channel_sync_plan(&sync_dir, &plan).await?;

    print_channel_sync_summary(
        &sync_dir,
        &run_summary.plan,
        run_summary.downloaded_count,
        run_summary.failed_count,
        false,
    );
    Ok(())
}

fn print_channel_sync_summary(
    sync_dir: &std::path::Path,
    plan_summary: &crate::channel_sync::ChannelSyncPlanSummary,
    downloaded_count: usize,
    failed_count: usize,
    dry_run: bool,
) {
    println!("sync-dir={}", sync_dir.display());
    println!("channel-dry-run={dry_run}");
    println!("channel-target-count={}", plan_summary.target_count);
    println!(
        "channel-discovered-video-count={}",
        plan_summary.discovered_video_count
    );
    println!(
        "channel-already-on-disk-count={}",
        plan_summary.already_on_disk_count
    );
    println!(
        "channel-pending-request-count={}",
        plan_summary.pending_request_count
    );
    println!(
        "channel-blocked-failure-count={}",
        plan_summary.blocked_failure_count
    );
    println!(
        "channel-new-request-count={}",
        plan_summary.new_request_count
    );
    println!(
        "channel-download-planned-count={}",
        plan_summary.download_planned_count
    );
    println!("channel-downloaded-count={downloaded_count}");
    println!("channel-failed-count={failed_count}");
}
