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
    candidate_video_count: usize,
    existing_video_count: usize,
    missing_video_count: usize,
    planned_video_count: usize,
}

#[derive(Facet, Debug)]
#[facet(rename_all = "kebab-case")]
struct SyncStatusThumbnailReport {
    candidate_video_count: usize,
    source_video_count: usize,
    discovered_count: usize,
    existing_count: usize,
    unavailable_count: usize,
    work_item_count: usize,
}

#[derive(Facet, Debug)]
#[facet(rename_all = "kebab-case")]
struct SyncStatusChannelReport {
    target_count: usize,
    discovered_video_count: usize,
    already_on_disk_count: usize,
    pending_request_count: usize,
    blocked_failure_count: usize,
    new_request_count: usize,
    download_planned_count: usize,
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

        Ok(CliOutput::facet(SyncStatusReport {
            sync_dir: sync_dir.display().to_string(),
            mft: SyncStatusMftReport {
                drive_count: mft_status.drives.len(),
                query_ready_drive_count: mft_status.query_ready_drive_count(),
                oldest_query_ready_at: teamy_mft::status::format_optional_system_time(
                    mft_status.oldest_query_ready_at(),
                ),
                oldest_query_ready_age: teamy_mft::status::format_optional_duration(
                    mft_status.oldest_query_ready_age(SystemTime::now()),
                ),
            },
            fetch: SyncStatusFetchReport {
                candidate_video_count: fetch_summary.candidate_videos,
                existing_video_count: fetch_summary.existing_fetches,
                missing_video_count: fetch_summary.missing_fetches,
                planned_video_count: fetch_summary.planned_fetches,
            },
            thumbnails: SyncStatusThumbnailReport {
                candidate_video_count: thumbnail_summary.candidate_videos,
                source_video_count: thumbnail_summary.source_videos,
                discovered_count: thumbnail_summary.discovered,
                existing_count: thumbnail_summary.existing,
                unavailable_count: thumbnail_summary.unavailable,
                work_item_count: thumbnail_summary.work_items,
            },
            channel: SyncStatusChannelReport {
                target_count: channel_plan.summary.target_count,
                discovered_video_count: channel_plan.summary.discovered_video_count,
                already_on_disk_count: channel_plan.summary.already_on_disk_count,
                pending_request_count: channel_plan.summary.pending_request_count,
                blocked_failure_count: channel_plan.summary.blocked_failure_count,
                new_request_count: channel_plan.summary.new_request_count,
                download_planned_count: channel_plan.summary.download_planned_count,
            },
        }))
    }
}
