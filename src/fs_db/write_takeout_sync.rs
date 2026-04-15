use crate::fs_db::SyncDatabaseSummary;
use crate::fs_db::VideoEventFile;
use crate::sync_progress::SyncProgress;
use crate::takeout::PlaylistVideoEntry;
use crate::takeout::WatchHistoryEntry;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use std::time::Instant;

const TAKEOUT_PROGRESS_LOG_INTERVAL: Duration = Duration::from_secs(1);

struct TakeoutWriteContext<'a> {
    sync_dir: &'a Path,
    imported_at: &'a str,
    watch_history_source_path: &'a Path,
    dry_run: bool,
    started_at: Instant,
}

#[derive(Debug)]
struct TakeoutProgressLogState {
    last_logged_at: Instant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WriteEventOutcome {
    DryRun,
    Existing,
    Written,
}

/// Write generic sync-database event files from parsed takeout inputs.
///
/// # Errors
///
/// Returns an error if any required directory or event file cannot be created.
pub async fn write_takeout_sync(
    sync_dir: &Path,
    imported_at: &str,
    watch_history_source_path: &Path,
    playlist_entries: &[PlaylistVideoEntry],
    watch_history_entries: &[WatchHistoryEntry],
    dry_run: bool,
) -> eyre::Result<SyncDatabaseSummary> {
    let started_at = Instant::now();
    let context = TakeoutWriteContext {
        sync_dir,
        imported_at,
        watch_history_source_path,
        dry_run,
        started_at,
    };
    let mut summary = build_sync_summary(playlist_entries, watch_history_entries);
    let mut progress = SyncProgress::new(playlist_entries.len() + watch_history_entries.len());
    let mut progress_log_state = TakeoutProgressLogState {
        last_logged_at: started_at,
    };
    let video_details_by_id = build_video_details_by_id(watch_history_entries);

    write_watch_history_events(
        &context,
        watch_history_entries,
        &mut summary,
        &mut progress,
        &mut progress_log_state,
    )
    .await?;
    write_playlist_events(
        &context,
        playlist_entries,
        &video_details_by_id,
        &mut summary,
        &mut progress,
        &mut progress_log_state,
    )
    .await?;

    Ok(summary)
}

fn build_sync_summary(
    playlist_entries: &[PlaylistVideoEntry],
    watch_history_entries: &[WatchHistoryEntry],
) -> SyncDatabaseSummary {
    SyncDatabaseSummary {
        unique_video_count: playlist_entries
            .iter()
            .map(|entry| entry.video_id.as_str().to_owned())
            .chain(
                watch_history_entries
                    .iter()
                    .map(|entry| entry.video_id.as_str().to_owned()),
            )
            .collect::<BTreeSet<_>>()
            .len(),
        unique_playlist_count: playlist_entries
            .iter()
            .map(|entry| entry.playlist_id.clone())
            .collect::<BTreeSet<_>>()
            .len(),
        playlist_event_count: playlist_entries.len(),
        watch_event_count: watch_history_entries.len(),
        written_event_file_count: 0,
        existing_event_file_count: 0,
    }
}

fn build_video_details_by_id(
    watch_history_entries: &[WatchHistoryEntry],
) -> HashMap<String, (Option<String>, Option<String>)> {
    let mut video_details_by_id = HashMap::new();
    for entry in watch_history_entries {
        video_details_by_id.insert(
            entry.video_id.as_str().to_owned(),
            (Some(entry.title.clone()), entry.channel_name.clone()),
        );
    }
    video_details_by_id
}

async fn write_watch_history_events(
    context: &TakeoutWriteContext<'_>,
    watch_history_entries: &[WatchHistoryEntry],
    summary: &mut SyncDatabaseSummary,
    progress: &mut SyncProgress,
    progress_log_state: &mut TakeoutProgressLogState,
) -> eyre::Result<()> {
    for entry in watch_history_entries {
        let event_file = VideoEventFile {
            imported_at: context.imported_at.to_owned(),
            source_kind: "takeout-watch-history".to_owned(),
            source_path: context.watch_history_source_path.display().to_string(),
            video_id: entry.video_id.as_str().to_owned(),
            video_title: Some(entry.title.clone()),
            channel_name: entry.channel_name.clone(),
            event_kind: "watched".to_owned(),
            event_at: entry.watched_at.to_rfc3339(),
            playlist_id: None,
            playlist_name: None,
        };
        let event_path = crate::fs_db::event_path_for(
            context.sync_dir,
            event_file.channel_name.as_deref(),
            Some(&entry.title),
            entry.video_id.as_str(),
            &event_file.event_at,
            "watched",
        );
        write_serialized_event_file(
            &event_path,
            &event_file,
            context.dry_run,
            summary,
            progress,
            context.started_at,
            progress_log_state,
        )
        .await?;
    }

    Ok(())
}

async fn write_playlist_events(
    context: &TakeoutWriteContext<'_>,
    playlist_entries: &[PlaylistVideoEntry],
    video_details_by_id: &HashMap<String, (Option<String>, Option<String>)>,
    summary: &mut SyncDatabaseSummary,
    progress: &mut SyncProgress,
    progress_log_state: &mut TakeoutProgressLogState,
) -> eyre::Result<()> {
    for entry in playlist_entries {
        let (video_title, channel_name) = video_details_by_id
            .get(entry.video_id.as_str())
            .cloned()
            .unwrap_or((None, None));
        let event_suffix = crate::fs_db::playlist_event_suffix(&entry.playlist_id);
        let event_file = VideoEventFile {
            imported_at: context.imported_at.to_owned(),
            source_kind: "takeout-playlist-membership".to_owned(),
            source_path: entry.source_file.clone(),
            video_id: entry.video_id.as_str().to_owned(),
            video_title: video_title.clone(),
            channel_name: channel_name.clone(),
            event_kind: "added-to-playlist".to_owned(),
            event_at: entry.added_at.to_rfc3339(),
            playlist_id: Some(entry.playlist_id.clone()),
            playlist_name: Some(entry.playlist_name.clone()),
        };
        let event_path = crate::fs_db::event_path_for(
            context.sync_dir,
            channel_name.as_deref(),
            video_title.as_deref(),
            entry.video_id.as_str(),
            &event_file.event_at,
            &event_suffix,
        );
        write_serialized_event_file(
            &event_path,
            &event_file,
            context.dry_run,
            summary,
            progress,
            context.started_at,
            progress_log_state,
        )
        .await?;
    }

    Ok(())
}

async fn write_serialized_event_file(
    path: &Path,
    event_file: &VideoEventFile,
    dry_run: bool,
    summary: &mut SyncDatabaseSummary,
    progress: &mut SyncProgress,
    started_at: Instant,
    progress_log_state: &mut TakeoutProgressLogState,
) -> eyre::Result<()> {
    let content = facet_json::to_string_pretty(event_file)?;
    write_event_file(
        path,
        &content,
        dry_run,
        summary,
        progress,
        started_at,
        progress_log_state,
    )
    .await
}

async fn write_event_file(
    path: &Path,
    content: &str,
    dry_run: bool,
    summary: &mut SyncDatabaseSummary,
    progress: &mut SyncProgress,
    started_at: Instant,
    progress_log_state: &mut TakeoutProgressLogState,
) -> eyre::Result<()> {
    let processed_bytes = u64::try_from(content.len())?;
    if dry_run {
        summary.written_event_file_count += 1;
        progress.record_item(processed_bytes, Some(path.display().to_string()));
        emit_takeout_progress_log(
            progress,
            started_at,
            WriteEventOutcome::DryRun,
            progress_log_state,
        );
        return Ok(());
    }

    if tokio::fs::try_exists(path).await? {
        summary.existing_event_file_count += 1;
        progress.record_item(processed_bytes, Some(path.display().to_string()));
        emit_takeout_progress_log(
            progress,
            started_at,
            WriteEventOutcome::Existing,
            progress_log_state,
        );
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    tokio::fs::write(path, content).await?;
    summary.written_event_file_count += 1;
    progress.record_item(processed_bytes, Some(path.display().to_string()));
    emit_takeout_progress_log(
        progress,
        started_at,
        WriteEventOutcome::Written,
        progress_log_state,
    );
    Ok(())
}

fn emit_takeout_progress_log(
    progress: &SyncProgress,
    started_at: Instant,
    outcome: WriteEventOutcome,
    progress_log_state: &mut TakeoutProgressLogState,
) {
    let now = Instant::now();
    if should_emit_takeout_progress_log(
        progress,
        outcome,
        now.duration_since(progress_log_state.last_logged_at),
    ) {
        progress.emit_log("sync takeout progress", started_at.elapsed());
        progress_log_state.last_logged_at = now;
    }
}

fn should_emit_takeout_progress_log(
    progress: &SyncProgress,
    outcome: WriteEventOutcome,
    time_since_last_log: Duration,
) -> bool {
    if !matches!(
        outcome,
        WriteEventOutcome::DryRun | WriteEventOutcome::Written
    ) {
        return false;
    }

    progress.items_processed() == progress.items_total()
        || time_since_last_log >= TAKEOUT_PROGRESS_LOG_INTERVAL
}

#[cfg(test)]
mod tests {
    use super::TAKEOUT_PROGRESS_LOG_INTERVAL;
    use super::WriteEventOutcome;
    use super::should_emit_takeout_progress_log;
    use crate::sync_progress::SyncProgress;
    use std::time::Duration;

    #[test]
    fn does_not_emit_progress_log_for_existing_event_files() {
        let progress = SyncProgress::new(10);
        assert!(!should_emit_takeout_progress_log(
            &progress,
            WriteEventOutcome::Existing,
            TAKEOUT_PROGRESS_LOG_INTERVAL
        ));
    }

    #[test]
    fn emits_progress_log_after_takeout_progress_interval() {
        let mut progress = SyncProgress::new(10);
        progress.record_item(1, None);

        assert!(should_emit_takeout_progress_log(
            &progress,
            WriteEventOutcome::Written,
            TAKEOUT_PROGRESS_LOG_INTERVAL
        ));
        assert!(should_emit_takeout_progress_log(
            &progress,
            WriteEventOutcome::DryRun,
            TAKEOUT_PROGRESS_LOG_INTERVAL
        ));
    }

    #[test]
    fn emits_progress_log_on_final_takeout_item_even_off_interval() {
        let mut progress = SyncProgress::new(3);
        for _ in 0..3 {
            progress.record_item(1, None);
        }

        assert!(should_emit_takeout_progress_log(
            &progress,
            WriteEventOutcome::Written,
            Duration::ZERO
        ));
    }

    #[test]
    fn skips_progress_log_before_takeout_progress_interval() {
        let mut progress = SyncProgress::new(10);
        progress.record_item(1, None);

        assert!(!should_emit_takeout_progress_log(
            &progress,
            WriteEventOutcome::Written,
            TAKEOUT_PROGRESS_LOG_INTERVAL - Duration::from_millis(1)
        ));
    }
}
