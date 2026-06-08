use crate::cli::output::CliOutput;
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
    #[facet(args::named, default)]
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

#[derive(Facet, Debug)]
#[facet(rename_all = "kebab-case")]
struct SyncChannelSummaryReport {
    targets: usize,
    discovered_videos: usize,
    already_on_disk: usize,
    pending_requests: usize,
    blocked_failures: usize,
    new_requests: usize,
    downloads_planned: usize,
}

#[derive(Facet, Debug)]
#[facet(rename_all = "kebab-case")]
pub(crate) struct SyncChannelReport {
    sync_dir: String,
    dry_run: bool,
    summary: SyncChannelSummaryReport,
    downloaded_count: usize,
    failed_count: usize,
}

impl SyncChannelArgs {
    /// # Errors
    ///
    /// This function will return an error if discovery, queuing, teamy-mft checks,
    /// or video downloads fail.
    pub async fn invoke(self, mft_max_age: Option<Duration>) -> eyre::Result<CliOutput> {
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
            None => Ok(CliOutput::facet(
                run_channel_sync(mft_max_age, dry_run).await?,
            )),
        }
    }
}

async fn run_channel_sync(
    mft_max_age: Option<Duration>,
    dry_run: bool,
) -> eyre::Result<SyncChannelReport> {
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
        return Ok(build_channel_sync_report(
            &sync_dir,
            &plan.summary,
            0,
            0,
            true,
        ));
    }

    let run_summary = crate::channel_sync::execute_channel_sync_plan(&sync_dir, &plan).await?;

    Ok(build_channel_sync_report(
        &sync_dir,
        &run_summary.plan,
        run_summary.downloaded_count,
        run_summary.failed_count,
        false,
    ))
}

fn build_channel_sync_report(
    sync_dir: &std::path::Path,
    plan_summary: &crate::channel_sync::ChannelSyncPlanSummary,
    downloaded_count: usize,
    failed_count: usize,
    dry_run: bool,
) -> SyncChannelReport {
    SyncChannelReport {
        sync_dir: sync_dir.display().to_string(),
        dry_run,
        summary: SyncChannelSummaryReport {
            targets: plan_summary.target_count,
            discovered_videos: plan_summary.discovered_video_count,
            already_on_disk: plan_summary.already_on_disk_count,
            pending_requests: plan_summary.pending_request_count,
            blocked_failures: plan_summary.blocked_failure_count,
            new_requests: plan_summary.new_request_count,
            downloads_planned: plan_summary.download_planned_count,
        },
        downloaded_count,
        failed_count,
    }
}
