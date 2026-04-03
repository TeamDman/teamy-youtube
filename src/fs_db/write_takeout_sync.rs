use crate::fs_db::SyncDatabaseSummary;
use crate::fs_db::VideoEventFile;
use crate::sync_progress::SyncProgress;
use crate::takeout::PlaylistVideoEntry;
use crate::takeout::WatchHistoryEntry;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

struct TakeoutWriteContext<'a> {
    sync_dir: &'a Path,
    imported_at: &'a str,
    watch_history_source_path: &'a Path,
    dry_run: bool,
    started_at: Instant,
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
    let context = TakeoutWriteContext {
        sync_dir,
        imported_at,
        watch_history_source_path,
        dry_run,
        started_at: Instant::now(),
    };
    let mut summary = build_sync_summary(playlist_entries, watch_history_entries);
    let mut progress = SyncProgress::new(playlist_entries.len() + watch_history_entries.len());
    let video_details_by_id = build_video_details_by_id(watch_history_entries);

    write_watch_history_events(&context, watch_history_entries, &mut summary, &mut progress)
        .await?;
    write_playlist_events(
        &context,
        playlist_entries,
        &video_details_by_id,
        &mut summary,
        &mut progress,
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
) -> eyre::Result<()> {
    let content = facet_json::to_string_pretty(event_file)?;
    write_event_file(path, &content, dry_run, summary, progress, started_at).await
}

async fn write_event_file(
    path: &Path,
    content: &str,
    dry_run: bool,
    summary: &mut SyncDatabaseSummary,
    progress: &mut SyncProgress,
    started_at: Instant,
) -> eyre::Result<()> {
    let processed_bytes = u64::try_from(content.len())?;
    if dry_run {
        summary.written_event_file_count += 1;
        progress.record_item(processed_bytes, Some(path.display().to_string()));
        progress.emit_log("sync takeout progress", started_at.elapsed());
        return Ok(());
    }

    if tokio::fs::try_exists(path).await? {
        summary.existing_event_file_count += 1;
        progress.record_item(processed_bytes, Some(path.display().to_string()));
        progress.emit_log("sync takeout progress", started_at.elapsed());
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    tokio::fs::write(path, content).await?;
    summary.written_event_file_count += 1;
    progress.record_item(processed_bytes, Some(path.display().to_string()));
    progress.emit_log("sync takeout progress", started_at.elapsed());
    Ok(())
}
