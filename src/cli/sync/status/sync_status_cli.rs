use crate::cli::output::CliOutput;
use arbitrary::Arbitrary;
use facet::Facet;
use std::time::Duration;
use std::time::SystemTime;

/// Summarize remaining filesystem, metadata, thumbnail, and channel-download work.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct SyncStatusArgs;

#[derive(Facet, Debug)]
#[facet(rename_all = "kebab-case")]
struct SyncStatusMftReport {
    drive_count: usize,
    query_ready_drive_count: usize,
    oldest_query_ready_at: String,
    oldest_query_ready_age: String,
}

#[derive(Facet, Debug)]
#[facet(rename_all = "kebab-case")]
struct SyncStatusFetchReport {
    candidates: usize,
    existing: usize,
    missing: usize,
    planned: usize,
}

#[derive(Facet, Debug)]
#[facet(rename_all = "kebab-case")]
struct SyncStatusThumbnailReport {
    candidate_videos: usize,
    source_videos: usize,
    discovered: usize,
    existing: usize,
    unavailable: usize,
    work_items: usize,
}

#[derive(Facet, Debug)]
#[facet(rename_all = "kebab-case")]
struct SyncStatusChannelReport {
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
struct SyncStatusReport {
    sync_dir: String,
    mft: SyncStatusMftReport,
    fetch: SyncStatusFetchReport,
    thumbnails: SyncStatusThumbnailReport,
    channel: SyncStatusChannelReport,
}

impl SyncStatusArgs {
    /// # Errors
    ///
    /// This function will return an error if the sync dir is unset, teamy-mft-backed lookups
    /// fail, or the status planners cannot inspect current state.
    pub async fn invoke(self, mft_max_age: Option<Duration>) -> eyre::Result<CliOutput> {
        let sync_dir = crate::paths::try_get_sync_dir()?;
        crate::cli::sync::assert_teamy_mft_query_cache_fresh(mft_max_age)?;

        let mft_status = teamy_mft::status::TeamyMftStatus::load_all_drives()?;
        let fetch_summary =
            crate::cli::sync::run::fetch_videos::summarize_fetch_video_plan(&sync_dir)?;
        let thumbnail_summary =
            crate::cli::sync::run::thumbnails::summarize_thumbnail_plan(&sync_dir).await?;
        let channel_plan = crate::channel_sync::plan_channel_sync(
            &sync_dir,
            crate::channel_sync::ChannelSyncPlanMode::StatusOnly,
        )
        .await?;

        Ok(CliOutput::facet(
            SyncStatusReport {
                sync_dir: sync_dir.display().to_string(),
                mft: SyncStatusMftReport {
                    drive_count: mft_status.drives.len(),
                    query_ready_drive_count: mft_status.query_ready_drive_count(),
                    oldest_query_ready_at: format_optional_system_time(
                        mft_status.oldest_query_ready_at(),
                    ),
                    oldest_query_ready_age: format_optional_duration(
                        mft_status.oldest_query_ready_age(SystemTime::now()),
                    ),
                },
                fetch: SyncStatusFetchReport {
                    candidates: fetch_summary.candidate_videos,
                    existing: fetch_summary.existing_fetches,
                    missing: fetch_summary.missing_fetches,
                    planned: fetch_summary.planned_fetches,
                },
                thumbnails: SyncStatusThumbnailReport {
                    candidate_videos: thumbnail_summary.candidate_videos,
                    source_videos: thumbnail_summary.source_videos,
                    discovered: thumbnail_summary.discovered,
                    existing: thumbnail_summary.existing,
                    unavailable: thumbnail_summary.unavailable,
                    work_items: thumbnail_summary.work_items,
                },
                channel: SyncStatusChannelReport {
                    targets: channel_plan.summary.target_count,
                    discovered_videos: channel_plan.summary.discovered_video_count,
                    already_on_disk: channel_plan.summary.already_on_disk_count,
                    pending_requests: channel_plan.summary.pending_request_count,
                    blocked_failures: channel_plan.summary.blocked_failure_count,
                    new_requests: channel_plan.summary.new_request_count,
                    downloads_planned: channel_plan.summary.download_planned_count,
                },
            },
            Some(crate::elapsed_duration::budget(
                5000,
                fetch_summary
                    .candidate_videos
                    .saturating_add(thumbnail_summary.candidate_videos)
                    .saturating_add(channel_plan.summary.discovered_video_count),
                20,
            )), /* Inventory only; no network downloads. */
        ))
    }
}

fn format_optional_system_time(value: Option<SystemTime>) -> String {
    value.map_or_else(
        || "none".to_owned(),
        |inner| chrono::DateTime::<chrono::Utc>::from(inner).to_rfc3339(),
    )
}

fn format_optional_duration(value: Option<Duration>) -> String {
    value.map_or_else(
        || "none".to_owned(),
        |inner| humantime::format_duration(inner).to_string(),
    )
}
