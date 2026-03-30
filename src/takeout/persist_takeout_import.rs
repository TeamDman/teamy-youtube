use crate::paths::AppHome;
use crate::takeout::ImportSummary;
use crate::takeout::TakeoutImportManifest;
use crate::takeout::TakeoutWatchHistoryEventFile;
use crate::takeout::TakeoutWatchLaterEventFile;
use crate::takeout::WatchHistoryEntry;
use crate::takeout::WatchLaterEntry;
use chrono::DateTime;
use chrono::FixedOffset;
use chrono::Local;
use eyre::ContextCompat as _;
use eyre::WrapErr as _;
use std::path::Path;
use std::path::PathBuf;
use tokio::task::JoinSet;
use tracing::info;

const MAX_CONCURRENT_FILE_WRITES: usize = 64;

/// Persist a takeout import as immutable event files and a manifest under the app home.
///
/// # Errors
///
/// Returns an error if the import directory cannot be created or any event file cannot be written.
pub async fn persist_takeout_import(
    app_home: &AppHome,
    watch_later_csv: &Path,
    watch_history_json: &Path,
    watch_later_entries: &[WatchLaterEntry],
    watch_history_entries: &[WatchHistoryEntry],
    summary: &ImportSummary,
) -> eyre::Result<PathBuf> {
    let imported_at = Local::now();
    let import_directory = allocate_import_directory(app_home, imported_at).await?;

    tokio::fs::create_dir_all(&import_directory)
        .await
        .wrap_err_with(|| {
            format!(
                "failed to create import directory {}",
                import_directory.display()
            )
        })?;

    let manifest = TakeoutImportManifest::new(
        import_directory
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("unknown-import")
            .to_owned(),
        imported_at.to_rfc3339(),
        watch_later_csv.display().to_string(),
        watch_history_json.display().to_string(),
        summary.watch_later_entry_count,
        summary.watch_later_unique_video_count,
        summary.watch_history_entry_count,
        summary.watch_history_unique_video_count,
        summary.watch_history_skipped_entry_count,
        summary.overlap_video_count,
    );
    let manifest_path = import_directory.join("manifest.json");
    write_json_file(
        manifest_path,
        facet_json::to_vec_pretty(&manifest).wrap_err("failed to serialize import manifest")?,
    )
    .await?;

    let mut write_tasks = JoinSet::new();
    let imported_at_string = imported_at.to_rfc3339();

    for entry in watch_later_entries {
        let event = TakeoutWatchLaterEventFile::new(
            imported_at_string.clone(),
            watch_later_csv.display().to_string(),
            entry.video_id.as_str().to_owned(),
            "watch-later".to_owned(),
            entry.added_at.to_rfc3339(),
        );
        let event_path = import_directory
            .join("watch-later")
            .join(entry.video_id.as_str())
            .join(format!(
                "{}.added.json",
                file_safe_timestamp(entry.added_at)
            ));
        spawn_write(
            &mut write_tasks,
            event_path,
            facet_json::to_vec_pretty(&event)?,
        )
        .await?;
    }

    for entry in watch_history_entries {
        let event = TakeoutWatchHistoryEventFile::new(
            imported_at_string.clone(),
            watch_history_json.display().to_string(),
            entry.video_id.as_str().to_owned(),
            entry.title.clone(),
            entry.channel_name.clone(),
            entry.watched_at.to_rfc3339(),
        );
        let event_path = import_directory
            .join("watch-history")
            .join(entry.video_id.as_str())
            .join(format!(
                "{}.watched.json",
                file_safe_timestamp(entry.watched_at)
            ));
        spawn_write(
            &mut write_tasks,
            event_path,
            facet_json::to_vec_pretty(&event)?,
        )
        .await?;
    }

    while let Some(result) = write_tasks.join_next().await {
        result.wrap_err("takeout import write task panicked")??;
    }

    info!(
        import_directory = %import_directory.display(),
        watch_later_entry_count = summary.watch_later_entry_count,
        watch_history_entry_count = summary.watch_history_entry_count,
        "persisted takeout import"
    );
    Ok(import_directory)
}

async fn allocate_import_directory(
    app_home: &AppHome,
    imported_at: DateTime<Local>,
) -> eyre::Result<PathBuf> {
    let parent_directory = app_home.file_path("imports").join("takeout");
    tokio::fs::create_dir_all(&parent_directory)
        .await
        .wrap_err_with(|| format!("failed to create {}", parent_directory.display()))?;

    let base_name = imported_at.format("%Y-%m-%d_%H-%M-%S%.3f").to_string();
    for suffix in 0..1000 {
        let candidate = if suffix == 0 {
            parent_directory.join(&base_name)
        } else {
            parent_directory.join(format!("{base_name}_{suffix:03}"))
        };
        if !tokio::fs::try_exists(&candidate)
            .await
            .wrap_err_with(|| format!("failed checking {}", candidate.display()))?
        {
            return Ok(candidate);
        }
    }

    eyre::bail!(
        "failed to allocate a unique import directory under {}",
        parent_directory.display()
    );
}

async fn spawn_write(
    write_tasks: &mut JoinSet<eyre::Result<()>>,
    path: PathBuf,
    bytes: Vec<u8>,
) -> eyre::Result<()> {
    write_tasks.spawn(async move { write_json_file(path, bytes).await });

    if write_tasks.len() >= MAX_CONCURRENT_FILE_WRITES {
        let result = write_tasks
            .join_next()
            .await
            .expect("join set should contain at least one task");
        result.wrap_err("takeout import write task panicked")??;
    }

    Ok(())
}

async fn write_json_file(path: PathBuf, bytes: Vec<u8>) -> eyre::Result<()> {
    let parent = path
        .parent()
        .wrap_err_with(|| format!("{} has no parent directory", path.display()))?;
    tokio::fs::create_dir_all(parent)
        .await
        .wrap_err_with(|| format!("failed to create {}", parent.display()))?;
    tokio::fs::write(&path, bytes)
        .await
        .wrap_err_with(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn file_safe_timestamp(timestamp: DateTime<FixedOffset>) -> String {
    timestamp.format("%Y-%m-%dT%H-%M-%S%.3f%z").to_string()
}

#[cfg(test)]
mod tests {
    use super::persist_takeout_import;
    use crate::paths::AppHome;
    use crate::takeout::ImportSummary;
    use crate::takeout::TakeoutImportManifest;
    use crate::takeout::WatchHistoryEntry;
    use crate::takeout::WatchLaterEntry;
    use crate::takeout::YouTubeVideoId;
    use chrono::DateTime;
    use chrono::FixedOffset;

    #[tokio::test]
    async fn writes_manifest_and_event_files() {
        let test_directory = std::env::temp_dir().join(format!(
            "teamy-youtube-takeout-test-{}",
            chrono::Local::now().format("%Y%m%d%H%M%S%3f")
        ));
        let app_home = AppHome(test_directory.clone());

        let watch_later_entries = vec![WatchLaterEntry {
            video_id: YouTubeVideoId::new("watch-later-video").unwrap(),
            added_at: DateTime::parse_from_rfc3339("2026-03-26T17:55:54+00:00").unwrap(),
        }];
        let watch_history_entries = vec![WatchHistoryEntry {
            video_id: YouTubeVideoId::new("watch-history-video").unwrap(),
            title: "History title".to_owned(),
            channel_name: Some("History channel".to_owned()),
            watched_at: DateTime::<FixedOffset>::parse_from_rfc3339("2026-03-26T18:55:54+00:00")
                .unwrap(),
        }];
        let summary = ImportSummary::from_entries(&watch_later_entries, &watch_history_entries, 2);

        let import_directory = persist_takeout_import(
            &app_home,
            std::path::Path::new("watch-later.csv"),
            std::path::Path::new("watch-history.json"),
            &watch_later_entries,
            &watch_history_entries,
            &summary,
        )
        .await
        .unwrap();

        let manifest_path = import_directory.join("manifest.json");
        assert!(manifest_path.exists());
        let manifest_json = tokio::fs::read_to_string(&manifest_path).await.unwrap();
        let manifest: TakeoutImportManifest = facet_json::from_str(&manifest_json).unwrap();
        assert_eq!(manifest.watch_later_entry_count, 1);
        assert_eq!(manifest.watch_history_skipped_entry_count, 2);

        let watch_later_directory = import_directory
            .join("watch-later")
            .join("watch-later-video");
        let watch_history_directory = import_directory
            .join("watch-history")
            .join("watch-history-video");
        assert!(watch_later_directory.exists());
        assert!(watch_history_directory.exists());

        let _ = tokio::fs::remove_dir_all(test_directory).await;
    }
}
