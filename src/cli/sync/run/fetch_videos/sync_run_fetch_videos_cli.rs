use arbitrary::Arbitrary;
use chrono::Local;
use facet::Facet;
use figue as args;
use std::path::Path;
use std::time::Instant;
use tracing::debug;
use tracing::info;

/// Fetch missing video data for videos already referenced in the sync database.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct SyncRunFetchVideosArgs {
    /// Maximum number of missing videos to fetch during this run.
    #[facet(args::named)]
    pub fetch_limit: Option<usize>,
}

#[derive(Debug, Eq, PartialEq)]
struct FetchVideoPlan {
    missing_video_ids: Vec<crate::takeout::YouTubeVideoId>,
    existing_fetch_count: usize,
    total_missing_video_count: usize,
    skipped_due_to_limit_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FetchVideoPlanSummary {
    pub candidate_video_count: usize,
    pub existing_fetch_count: usize,
    pub missing_fetch_count: usize,
    pub fetch_planned_video_count: usize,
    pub skipped_due_to_limit_count: usize,
}

#[derive(Debug, Default, Eq, PartialEq)]
struct FetchVideoRunCounts {
    fetched: usize,
    missing: usize,
    unavailable: usize,
}

impl SyncRunFetchVideosArgs {
    /// # Errors
    ///
    /// This function will return an error if the sync dir is unset, the API key is unavailable,
    /// video discovery fails, the API request fails, or fetched files cannot be written.
    pub async fn invoke(self) -> eyre::Result<()> {
        let sync_dir = crate::paths::try_get_sync_dir()?;
        let api_key = crate::paths::try_get_youtube_api_key()?;
        let plan = build_fetch_video_plan(&sync_dir, self.fetch_limit)?;
        log_fetch_video_plan(&sync_dir, &plan, self.fetch_limit);

        let started_at = Instant::now();
        let mut progress = crate::sync_progress::SyncProgress::new(plan.missing_video_ids.len());
        let counts = run_fetch_video_plan(
            &sync_dir,
            &api_key,
            &plan.missing_video_ids,
            started_at,
            &mut progress,
        )
        .await?;

        print_fetch_video_summary(&sync_dir, &plan, self.fetch_limit, &counts);
        Ok(())
    }
}

fn build_fetch_video_plan(
    sync_dir: &Path,
    fetch_limit: Option<usize>,
) -> eyre::Result<FetchVideoPlan> {
    let candidate_video_ids = crate::fs_db::load_video_ids_from_sync_dir(sync_dir)?;
    let mut missing_video_ids = Vec::new();
    let mut existing_fetch_count = 0_usize;

    for video_id in candidate_video_ids {
        if crate::fs_db::has_terminal_video_fetch_event(sync_dir, video_id.as_str())? {
            existing_fetch_count += 1;
        } else {
            missing_video_ids.push(video_id);
        }
    }

    let total_missing_video_count = missing_video_ids.len();
    if let Some(fetch_limit) = fetch_limit {
        missing_video_ids.truncate(fetch_limit);
    }

    Ok(FetchVideoPlan {
        skipped_due_to_limit_count: total_missing_video_count - missing_video_ids.len(),
        missing_video_ids,
        existing_fetch_count,
        total_missing_video_count,
    })
}

pub(crate) fn summarize_fetch_video_plan(sync_dir: &Path) -> eyre::Result<FetchVideoPlanSummary> {
    let plan = build_fetch_video_plan(sync_dir, None)?;
    Ok(FetchVideoPlanSummary {
        candidate_video_count: plan.existing_fetch_count + plan.total_missing_video_count,
        existing_fetch_count: plan.existing_fetch_count,
        missing_fetch_count: plan.total_missing_video_count,
        fetch_planned_video_count: plan.missing_video_ids.len(),
        skipped_due_to_limit_count: plan.skipped_due_to_limit_count,
    })
}

fn log_fetch_video_plan(sync_dir: &Path, plan: &FetchVideoPlan, fetch_limit: Option<usize>) {
    info!(
        sync_dir = %sync_dir.display(),
        candidate_video_count = plan.existing_fetch_count + plan.total_missing_video_count,
        existing_fetch_video_count = plan.existing_fetch_count,
        missing_fetch_video_count = plan.total_missing_video_count,
        fetch_planned_video_count = plan.missing_video_ids.len(),
        fetch_limit = %format_optional_limit(fetch_limit),
        fetch_skipped_due_to_limit_count = plan.skipped_due_to_limit_count,
        "starting sync videos"
    );
}

async fn run_fetch_video_plan(
    sync_dir: &Path,
    api_key: &str,
    missing_video_ids: &[crate::takeout::YouTubeVideoId],
    started_at: Instant,
    progress: &mut crate::sync_progress::SyncProgress,
) -> eyre::Result<FetchVideoRunCounts> {
    let mut counts = FetchVideoRunCounts::default();

    for video_id in missing_video_ids {
        process_single_video_fetch(
            sync_dir,
            api_key,
            video_id,
            progress,
            started_at,
            &mut counts,
        )
        .await?;
    }

    Ok(counts)
}

async fn process_single_video_fetch(
    sync_dir: &Path,
    api_key: &str,
    video_id: &crate::takeout::YouTubeVideoId,
    progress: &mut crate::sync_progress::SyncProgress,
    started_at: Instant,
    counts: &mut FetchVideoRunCounts,
) -> eyre::Result<()> {
    let fetched_at = Local::now().to_rfc3339();
    match crate::youtube_api::fetch_video_data(video_id, api_key).await? {
        crate::youtube_api::YouTubeVideoFetchOutcome::Found(metadata) => {
            let (fetch_event_path, title_observation_path) =
                crate::fs_db::write_fetched_video_data(sync_dir, &fetched_at, &metadata).await?;
            let written_bytes = u64::try_from(metadata.raw_response_body.as_str().len())?
                + u64::try_from(metadata.title.len())?;
            progress.record_item(written_bytes, Some(fetch_event_path.display().to_string()));
            debug!(
                video_id = %metadata.video_id,
                fetch_event_path = %fetch_event_path.display(),
                title_observation_path = %title_observation_path.display(),
                written_bytes,
                written_bytes_human = %crate::sync_progress::format_bytes(written_bytes),
                "finished writing fetched video files"
            );
            progress.emit_log("sync videos progress", started_at.elapsed());
            counts.fetched += 1;
        }
        crate::youtube_api::YouTubeVideoFetchOutcome::Missing {
            video_id,
            raw_response_body,
            ..
        } => {
            let path = crate::fs_db::write_missing_video_data(
                sync_dir,
                &fetched_at,
                &video_id,
                "fetch_video_data_missing",
                &raw_response_body,
            )
            .await?;
            let written_bytes = u64::try_from(raw_response_body.as_str().len())?;
            progress.record_item(written_bytes, Some(path.display().to_string()));
            debug!(
                video_id,
                event_path = %path.display(),
                written_bytes,
                written_bytes_human = %crate::sync_progress::format_bytes(written_bytes),
                "finished writing missing video result"
            );
            progress.emit_log("sync videos progress", started_at.elapsed());
            counts.missing += 1;
        }
        crate::youtube_api::YouTubeVideoFetchOutcome::Unavailable {
            video_id,
            raw_response_body,
            ..
        } => {
            let path = crate::fs_db::write_missing_video_data(
                sync_dir,
                &fetched_at,
                &video_id,
                "fetch_video_data_unavailable",
                &raw_response_body,
            )
            .await?;
            let written_bytes = u64::try_from(raw_response_body.as_str().len())?;
            progress.record_item(written_bytes, Some(path.display().to_string()));
            debug!(
                video_id,
                event_path = %path.display(),
                written_bytes,
                written_bytes_human = %crate::sync_progress::format_bytes(written_bytes),
                "finished writing unavailable video result"
            );
            progress.emit_log("sync videos progress", started_at.elapsed());
            counts.unavailable += 1;
        }
    }

    Ok(())
}

fn print_fetch_video_summary(
    sync_dir: &Path,
    plan: &FetchVideoPlan,
    fetch_limit: Option<usize>,
    counts: &FetchVideoRunCounts,
) {
    println!("sync-dir={}", sync_dir.display());
    println!(
        "candidate-video-count={}",
        plan.existing_fetch_count + plan.total_missing_video_count
    );
    println!("existing-fetch-video-count={}", plan.existing_fetch_count);
    println!(
        "missing-fetch-video-count={}",
        plan.total_missing_video_count
    );
    println!("fetch-planned-video-count={}", plan.missing_video_ids.len());
    println!("fetch-completed-video-count={}", counts.fetched);
    println!("fetch-missing-result-count={}", counts.missing);
    println!("fetch-unavailable-result-count={}", counts.unavailable);
    println!("fetch-limit={}", format_optional_limit(fetch_limit));
    println!(
        "fetch-skipped-due-to-limit-count={}",
        plan.skipped_due_to_limit_count
    );
}

fn format_optional_limit(value: Option<usize>) -> String {
    value.map_or_else(|| "none".to_owned(), |inner| inner.to_string())
}

#[cfg(test)]
mod tests {
    use super::build_fetch_video_plan;
    use super::format_optional_limit;
    use crate::fs_db::video_title_observation_path_for;
    use std::path::Path;
    use tempfile::TempDir;

    #[test]
    fn formats_missing_fetch_limit() {
        assert_eq!(format_optional_limit(None), "none");
    }

    #[test]
    fn formats_present_fetch_limit() {
        assert_eq!(format_optional_limit(Some(25)), "25");
    }

    #[test]
    fn formats_human_bytes() {
        assert_eq!(crate::sync_progress::format_bytes(999), "999 B");
        assert_eq!(crate::sync_progress::format_bytes(2_048), "2.0 KiB");
    }

    #[test]
    fn unicode_title_observation_path_does_not_panic() {
        let path = video_title_observation_path_for(
            Path::new("G:/sync-root"),
            "abc123",
            "2026-04-03T12:34:56+00:00",
            "【東方ヴォーカルPV】LOVE EAST【暁Records公式】",
        );

        assert!(path.display().to_string().contains("observe_title"));
    }

    #[test]
    fn fetch_video_plan_skips_already_terminal_videos() {
        let temp_dir = TempDir::new().expect("temp dir should be created");
        let video_dir = temp_dir.path().join("videos").join("abc123");
        std::fs::create_dir_all(&video_dir).expect("video dir should be created");
        let event_path = crate::fs_db::video_fetch_event_path_for(
            temp_dir.path(),
            "abc123",
            "2026-04-04T00:00:00+00:00",
        );
        std::fs::write(&event_path, "{}\n").expect("event file should be written");

        let plan = build_fetch_video_plan(temp_dir.path(), None).expect("plan should build");

        assert_eq!(plan.existing_fetch_count, 1);
        assert_eq!(plan.total_missing_video_count, 0);
        assert!(plan.missing_video_ids.is_empty());
    }

    #[test]
    fn fetch_video_plan_includes_only_missing_videos() {
        let temp_dir = TempDir::new().expect("temp dir should be created");
        std::fs::create_dir_all(temp_dir.path().join("videos").join("abc123"))
            .expect("first video dir should be created");
        std::fs::create_dir_all(temp_dir.path().join("videos").join("def456"))
            .expect("second video dir should be created");
        let event_path = crate::fs_db::video_fetch_event_path_for(
            temp_dir.path(),
            "abc123",
            "2026-04-04T00:00:00+00:00",
        );
        std::fs::write(&event_path, "{}\n").expect("event file should be written");

        let plan = build_fetch_video_plan(temp_dir.path(), None).expect("plan should build");

        assert_eq!(plan.existing_fetch_count, 1);
        assert_eq!(plan.total_missing_video_count, 1);
        assert_eq!(plan.missing_video_ids.len(), 1);
        assert_eq!(plan.missing_video_ids[0].as_str(), "def456");
    }
}
