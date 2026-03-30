use crate::fs_db::SyncDatabaseSummary;
use crate::fs_db::VideoEventFile;
use crate::takeout::PlaylistVideoEntry;
use crate::takeout::WatchHistoryEntry;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::path::Path;

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
    let mut summary = SyncDatabaseSummary {
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
    };

    let mut video_details_by_id = HashMap::new();
    for entry in watch_history_entries {
        video_details_by_id.insert(
            entry.video_id.as_str().to_owned(),
            (Some(entry.title.clone()), entry.channel_name.clone()),
        );
    }

    for entry in watch_history_entries {
        let event_file = VideoEventFile {
            imported_at: imported_at.to_owned(),
            source_kind: "takeout-watch-history".to_owned(),
            source_path: watch_history_source_path.display().to_string(),
            video_id: entry.video_id.as_str().to_owned(),
            video_title: Some(entry.title.clone()),
            channel_name: entry.channel_name.clone(),
            event_kind: "watched".to_owned(),
            event_at: entry.watched_at.to_rfc3339(),
            playlist_id: None,
            playlist_name: None,
        };
        let event_path = crate::fs_db::event_path_for(
            sync_dir,
            event_file.channel_name.as_deref(),
            Some(&entry.title),
            entry.video_id.as_str(),
            &event_file.event_at,
            "watched",
        );
        write_event_file(&event_path, &event_file, dry_run, &mut summary).await?;
    }

    for entry in playlist_entries {
        let (video_title, channel_name) = video_details_by_id
            .get(entry.video_id.as_str())
            .cloned()
            .unwrap_or((None, None));
        let event_suffix = crate::fs_db::playlist_event_suffix(&entry.playlist_id);
        let event_file = VideoEventFile {
            imported_at: imported_at.to_owned(),
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
            sync_dir,
            channel_name.as_deref(),
            video_title.as_deref(),
            entry.video_id.as_str(),
            &event_file.event_at,
            &event_suffix,
        );
        write_event_file(&event_path, &event_file, dry_run, &mut summary).await?;
    }

    Ok(summary)
}

async fn write_event_file(
    path: &Path,
    event_file: &VideoEventFile,
    dry_run: bool,
    summary: &mut SyncDatabaseSummary,
) -> eyre::Result<()> {
    if dry_run {
        summary.written_event_file_count += 1;
        return Ok(());
    }

    if tokio::fs::try_exists(path).await? {
        summary.existing_event_file_count += 1;
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let content = facet_json::to_string_pretty(event_file)?;
    tokio::fs::write(path, content).await?;
    summary.written_event_file_count += 1;
    Ok(())
}
