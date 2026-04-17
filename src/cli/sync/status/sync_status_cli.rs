use arbitrary::Arbitrary;
use facet::Facet;
use std::time::Duration;
use std::time::SystemTime;

/// Summarize remaining filesystem, metadata, thumbnail, and channel-download work.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct SyncStatusArgs;

impl SyncStatusArgs {
    /// # Errors
    ///
    /// This function will return an error if the sync dir is unset, teamy-mft-backed lookups
    /// fail, or the status planners cannot inspect current state.
    pub async fn invoke(self, mft_max_age: Option<Duration>) -> eyre::Result<()> {
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

        println!("sync-dir={}", sync_dir.display());
        println!("mft-drive-count={}", mft_status.drives.len());
        println!(
            "mft-query-ready-drive-count={}",
            mft_status.query_ready_drive_count()
        );
        println!(
            "mft-oldest-query-ready-at={}",
            teamy_mft::status::format_optional_system_time(mft_status.oldest_query_ready_at())
        );
        println!(
            "mft-oldest-query-ready-age={}",
            teamy_mft::status::format_optional_duration(
                mft_status.oldest_query_ready_age(SystemTime::now())
            )
        );

        println!(
            "fetch-candidate-video-count={}",
            fetch_summary.candidate_video_count
        );
        println!(
            "fetch-existing-video-count={}",
            fetch_summary.existing_fetch_count
        );
        println!(
            "fetch-missing-video-count={}",
            fetch_summary.missing_fetch_count
        );
        println!(
            "fetch-planned-video-count={}",
            fetch_summary.fetch_planned_video_count
        );

        println!(
            "thumbnail-candidate-video-count={}",
            thumbnail_summary.candidate_video_count
        );
        println!(
            "thumbnail-source-video-count={}",
            thumbnail_summary.source_video_count
        );
        println!(
            "thumbnail-discovered-count={}",
            thumbnail_summary.discovered_count
        );
        println!(
            "thumbnail-existing-count={}",
            thumbnail_summary.existing_count
        );
        println!(
            "thumbnail-unavailable-count={}",
            thumbnail_summary.unavailable_count
        );
        println!(
            "thumbnail-work-item-count={}",
            thumbnail_summary.work_item_count
        );

        println!("channel-target-count={}", channel_plan.summary.target_count);
        println!(
            "channel-discovered-video-count={}",
            channel_plan.summary.discovered_video_count
        );
        println!(
            "channel-already-on-disk-count={}",
            channel_plan.summary.already_on_disk_count
        );
        println!(
            "channel-pending-request-count={}",
            channel_plan.summary.pending_request_count
        );
        println!(
            "channel-blocked-failure-count={}",
            channel_plan.summary.blocked_failure_count
        );
        println!(
            "channel-new-request-count={}",
            channel_plan.summary.new_request_count
        );
        println!(
            "channel-download-planned-count={}",
            channel_plan.summary.download_planned_count
        );
        Ok(())
    }
}
