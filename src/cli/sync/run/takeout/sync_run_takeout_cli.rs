use arbitrary::Arbitrary;
use chrono::Local;
use facet::Facet;
use figue as args;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;
use teamy_mft::cli::command::query::QueryArgs;
use tracing::info;

/// Build the sync database from Google Takeout history and playlists.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct SyncRunTakeoutArgs {
    /// Print a summary without writing the sync database.
    #[facet(args::named, default)]
    pub dry_run: bool,

    /// Optional directory to scan instead of discovering the latest takeout via teamy-mft.
    #[facet(args::named)]
    pub input_dir: Option<String>,
}

impl SyncRunTakeoutArgs {
    /// # Errors
    ///
    /// This function will return an error if the sync dir is unset, discovery fails,
    /// the takeout inputs cannot be read, or the sync database cannot be written.
    pub async fn invoke(self) -> eyre::Result<()> {
        let sync_dir = crate::paths::try_get_sync_dir()?;
        let discovered_inputs = discover_inputs(self.input_dir)?;

        info!(
            sync_dir = %sync_dir.display(),
            watch_history_json = %discovered_inputs.watch_history_json.display(),
            playlist_csv_count = discovered_inputs.playlist_csvs.len(),
            dry_run = self.dry_run,
            "syncing from takeout inputs"
        );

        let playlist_entries = load_playlist_entries(&discovered_inputs.playlist_csvs).await?;
        let watch_history_report =
            crate::takeout::read_watch_history_entries(&discovered_inputs.watch_history_json)
                .await?;
        let imported_at = Local::now().to_rfc3339();
        let sync_summary = crate::fs_db::write_takeout_sync(
            &sync_dir,
            &imported_at,
            &discovered_inputs.watch_history_json,
            &playlist_entries,
            &watch_history_report.entries,
            self.dry_run,
        )
        .await?;

        print_sync_summary(
            &sync_dir,
            self.dry_run,
            &discovered_inputs,
            &playlist_entries,
            &watch_history_report,
            &sync_summary,
        );
        if self.dry_run {
            for line in build_dry_run_preview_lines(
                &sync_dir,
                &playlist_entries,
                &watch_history_report.entries,
            ) {
                println!("{line}");
            }
        }

        Ok(())
    }
}

struct DiscoveredInputs {
    watch_history_json: PathBuf,
    playlist_csvs: Vec<PathBuf>,
    source_label: String,
}

fn discover_inputs(input_dir: Option<String>) -> eyre::Result<DiscoveredInputs> {
    let (watch_history_json, playlist_csvs, source_label) = if let Some(input_dir) = input_dir {
        let input_dir = normalize_input_dir(&input_dir)?;
        let discovered = discover_takeout_inputs_from_directory(&input_dir)?;
        (discovered.0, discovered.1, input_dir.display().to_string())
    } else {
        let discovered = discover_takeout_inputs_from_index()?;
        (discovered.0, discovered.1, "teamy-mft-index".to_owned())
    };

    Ok(DiscoveredInputs {
        watch_history_json,
        playlist_csvs,
        source_label,
    })
}

async fn load_playlist_entries(
    playlist_csvs: &[PathBuf],
) -> eyre::Result<Vec<crate::takeout::PlaylistVideoEntry>> {
    let mut playlist_entries = Vec::new();
    for playlist_csv in playlist_csvs {
        let playlist_name = playlist_name_from_csv_path(playlist_csv)?;
        let mut entries =
            crate::takeout::read_playlist_video_entries(playlist_csv, playlist_name).await?;
        playlist_entries.append(&mut entries);
    }

    Ok(playlist_entries)
}

fn print_sync_summary(
    sync_dir: &Path,
    dry_run: bool,
    discovered_inputs: &DiscoveredInputs,
    playlist_entries: &[crate::takeout::PlaylistVideoEntry],
    watch_history_report: &crate::takeout::WatchHistoryReport,
    sync_summary: &crate::fs_db::SyncDatabaseSummary,
) {
    let unique_playlist_names = playlist_entries
        .iter()
        .map(|entry| entry.playlist_name.clone())
        .collect::<BTreeSet<_>>()
        .len();
    let unique_playlist_video_ids = playlist_entries
        .iter()
        .map(|entry| entry.video_id.as_str().to_owned())
        .collect::<BTreeSet<_>>()
        .len();
    let unique_watch_history_video_ids = watch_history_report
        .entries
        .iter()
        .map(|entry| entry.video_id.as_str().to_owned())
        .collect::<BTreeSet<_>>()
        .len();

    println!("source={}", discovered_inputs.source_label);
    println!("sync-dir={}", sync_dir.display());
    println!("dry-run={dry_run}");
    println!(
        "watch-history-json={}",
        discovered_inputs.watch_history_json.display()
    );
    println!(
        "playlist-csv-count={}",
        discovered_inputs.playlist_csvs.len()
    );
    println!("playlist-count={unique_playlist_names}");
    println!("playlist-entry-count={}", playlist_entries.len());
    println!("playlist-unique-video-ids={unique_playlist_video_ids}");
    println!(
        "watch-history-entry-count={}",
        watch_history_report.entries.len()
    );
    println!("watch-history-unique-video-ids={unique_watch_history_video_ids}");
    println!(
        "watch-history-skipped-entry-count={}",
        watch_history_report.skipped_entry_count
    );
    println!(
        "sync-unique-video-count={}",
        sync_summary.unique_video_count
    );
    println!(
        "sync-unique-playlist-count={}",
        sync_summary.unique_playlist_count
    );
    println!(
        "sync-playlist-event-count={}",
        sync_summary.playlist_event_count
    );
    println!("sync-watch-event-count={}", sync_summary.watch_event_count);
    println!(
        "sync-written-event-file-count={}",
        sync_summary.written_event_file_count
    );
    println!(
        "sync-existing-event-file-count={}",
        sync_summary.existing_event_file_count
    );
}

fn build_dry_run_preview_lines(
    sync_dir: &Path,
    playlist_entries: &[crate::takeout::PlaylistVideoEntry],
    watch_history_entries: &[crate::takeout::WatchHistoryEntry],
) -> Vec<String> {
    let Some(watch_entry) = watch_history_entries.iter().find(|watch_entry| {
        playlist_entries
            .iter()
            .any(|playlist_entry| playlist_entry.video_id == watch_entry.video_id)
    }) else {
        return vec!["preview-sample=unavailable-no-watch-playlist-overlap".to_owned()];
    };

    let matching_playlist_entries = playlist_entries
        .iter()
        .filter(|playlist_entry| playlist_entry.video_id == watch_entry.video_id)
        .collect::<Vec<_>>();
    let watched_event_path = crate::fs_db::event_path_for(
        sync_dir,
        watch_entry.channel_name.as_deref(),
        Some(&watch_entry.title),
        watch_entry.video_id.as_str(),
        &watch_entry.watched_at.to_rfc3339(),
        "watched",
    );
    let Some(video_dir) = watched_event_path.parent() else {
        return vec!["preview-sample=unavailable-invalid-event-path".to_owned()];
    };

    let mut lines = vec![
        format!("preview-sample-video-id={}", watch_entry.video_id.as_str()),
        format!("preview-sample-video-title={}", watch_entry.title),
        format!(
            "preview-sample-video-dir={}",
            format_preview_path(sync_dir, video_dir)
        ),
        format!(
            "preview-sample-event-file={}",
            format_preview_path(sync_dir, &watched_event_path)
        ),
    ];

    let mut playlist_event_paths = matching_playlist_entries
        .into_iter()
        .map(|playlist_entry| {
            crate::fs_db::event_path_for(
                sync_dir,
                watch_entry.channel_name.as_deref(),
                Some(&watch_entry.title),
                watch_entry.video_id.as_str(),
                &playlist_entry.added_at.to_rfc3339(),
                &crate::fs_db::playlist_event_suffix(&playlist_entry.playlist_id),
            )
        })
        .collect::<Vec<_>>();
    playlist_event_paths.sort();
    for playlist_event_path in playlist_event_paths {
        lines.push(format!(
            "preview-sample-event-file={}",
            format_preview_path(sync_dir, &playlist_event_path)
        ));
    }

    lines
}

fn format_preview_path(sync_dir: &Path, path: &Path) -> String {
    let display_path = path.strip_prefix(sync_dir).unwrap_or(path);
    display_path.display().to_string().replace('\\', "/")
}

fn normalize_input_dir(value: &str) -> eyre::Result<PathBuf> {
    let input_dir = PathBuf::from(value);
    if input_dir.is_absolute() {
        return Ok(input_dir);
    }

    Ok(std::env::current_dir()?.join(input_dir))
}

fn discover_takeout_inputs_from_directory(
    input_dir: &Path,
) -> eyre::Result<(PathBuf, Vec<PathBuf>)> {
    let mut watch_history_candidates = Vec::new();
    let mut playlist_candidates = Vec::new();
    collect_takeout_files(
        input_dir,
        &mut watch_history_candidates,
        &mut playlist_candidates,
    )?;
    select_takeout_inputs(watch_history_candidates, playlist_candidates)
}

fn discover_takeout_inputs_from_index() -> eyre::Result<(PathBuf, Vec<PathBuf>)> {
    let mut candidates = QueryArgs {
        query: vec!["takeout history".to_owned(), "takeout playlists".to_owned()],
        ..Default::default()
    }
    .invoke()?;

    candidates.sort();
    let mut watch_history_candidates = Vec::new();
    let mut playlist_candidates = Vec::new();
    for candidate in candidates {
        if is_watch_history_json(&candidate) {
            watch_history_candidates.push(candidate);
        } else if is_playlist_csv(&candidate) {
            playlist_candidates.push(candidate);
        }
    }

    select_takeout_inputs(watch_history_candidates, playlist_candidates)
}

fn select_takeout_inputs(
    watch_history_candidates: Vec<PathBuf>,
    playlist_candidates: Vec<PathBuf>,
) -> eyre::Result<(PathBuf, Vec<PathBuf>)> {
    let mut watch_history_candidates = watch_history_candidates;
    watch_history_candidates.sort_by_key(|path| std::cmp::Reverse(candidate_timestamp_key(path)));
    let watch_history_json = watch_history_candidates
        .into_iter()
        .find(|path| is_readable_file(path))
        .ok_or_else(|| {
            eyre::eyre!(
                "Could not find a readable watch-history.json in the selected takeout inputs"
            )
        })?;

    let mut newest_playlist_by_name = BTreeMap::new();
    for candidate in playlist_candidates {
        if !is_readable_file(&candidate) {
            continue;
        }
        let playlist_name = playlist_name_from_csv_path(&candidate)?;
        let key = candidate_timestamp_key(&candidate);
        match newest_playlist_by_name.get(&playlist_name) {
            Some((existing_key, _)) if *existing_key >= key => {}
            _ => {
                newest_playlist_by_name.insert(playlist_name, (key, candidate));
            }
        }
    }

    let playlist_csvs = newest_playlist_by_name
        .into_values()
        .map(|(_, candidate)| candidate)
        .collect::<Vec<_>>();
    if playlist_csvs.is_empty() {
        eyre::bail!("Could not find any playlist CSV files in the selected takeout inputs");
    }

    Ok((watch_history_json, playlist_csvs))
}

fn collect_takeout_files(
    directory: &Path,
    watch_history_candidates: &mut Vec<PathBuf>,
    playlist_candidates: &mut Vec<PathBuf>,
) -> eyre::Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect_takeout_files(&path, watch_history_candidates, playlist_candidates)?;
            continue;
        }

        if is_watch_history_json(&path) {
            watch_history_candidates.push(path);
        } else if is_playlist_csv(&path) {
            playlist_candidates.push(path);
        }
    }

    Ok(())
}

fn is_watch_history_json(path: &Path) -> bool {
    path.file_name()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|value| value.eq_ignore_ascii_case("watch-history.json"))
}

fn is_playlist_csv(path: &Path) -> bool {
    path.file_name()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|value| value.to_ascii_lowercase().ends_with("-videos.csv"))
}

fn playlist_name_from_csv_path(path: &Path) -> eyre::Result<String> {
    let file_name = path
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .ok_or_else(|| eyre::eyre!("playlist path has no file name: {}", path.display()))?;
    let playlist_name = file_name
        .strip_suffix("-videos.csv")
        .or_else(|| file_name.strip_suffix("-VIDEOS.CSV"))
        .unwrap_or(file_name)
        .replace('_', " ");
    Ok(playlist_name)
}

fn candidate_timestamp_key(path: &Path) -> String {
    if let Some(root) = takeout_root(path)
        && let Some(file_name) = root.file_name().and_then(std::ffi::OsStr::to_str)
    {
        let lowercase = file_name.to_ascii_lowercase();
        if let Some(remainder) = lowercase.strip_prefix("takeout-") {
            return remainder.split('-').next().unwrap_or_default().to_owned();
        }
    }

    path.display().to_string()
}

fn takeout_root(path: &Path) -> Option<PathBuf> {
    for ancestor in path.ancestors() {
        let Some(name) = ancestor.file_name().and_then(std::ffi::OsStr::to_str) else {
            continue;
        };
        if name.eq_ignore_ascii_case("Takeout") {
            return ancestor.parent().map(Path::to_path_buf);
        }
        if name.to_ascii_lowercase().starts_with("takeout-") {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

fn is_readable_file(path: &Path) -> bool {
    std::fs::read(path).is_ok()
}

#[cfg(test)]
mod tests {
    use super::build_dry_run_preview_lines;
    use chrono::DateTime;
    use chrono::FixedOffset;
    use std::path::Path;

    #[test]
    fn dry_run_preview_includes_watch_and_playlist_paths_for_overlap() {
        let playlist_entries = vec![crate::takeout::PlaylistVideoEntry {
            playlist_id: "favorites".to_owned(),
            playlist_name: "Favorites".to_owned(),
            source_file: "favorites.csv".to_owned(),
            video_id: crate::takeout::YouTubeVideoId::new("XfcLWVX-hCA")
                .expect("video id should parse"),
            added_at: DateTime::<FixedOffset>::parse_from_rfc3339("2026-02-20T18:22:41+00:00")
                .expect("playlist timestamp should parse"),
        }];
        let watch_history_entries = vec![crate::takeout::WatchHistoryEntry {
            video_id: crate::takeout::YouTubeVideoId::new("XfcLWVX-hCA")
                .expect("video id should parse"),
            title: "Watched Arc Raiders War Tapes 1".to_owned(),
            channel_name: Some("0biwankenobi".to_owned()),
            watched_at: DateTime::<FixedOffset>::parse_from_rfc3339(
                "2025-11-20T23:03:24.580+00:00",
            )
            .expect("watch timestamp should parse"),
        }];

        let lines = build_dry_run_preview_lines(
            Path::new("G:/sync-root"),
            &playlist_entries,
            &watch_history_entries,
        );

        assert_eq!(lines[0], "preview-sample-video-id=XfcLWVX-hCA");
        assert_eq!(
            lines[1],
            "preview-sample-video-title=Watched Arc Raiders War Tapes 1"
        );
        assert_eq!(
            lines[2],
            "preview-sample-video-dir=channels/0biwankenobi/videos/XfcLWVX-hCA-arc-raiders-war-tapes-1"
        );
        assert_eq!(
            lines[3],
            "preview-sample-event-file=channels/0biwankenobi/videos/XfcLWVX-hCA-arc-raiders-war-tapes-1/event_2025-11-20T23-03-24.580+00-00_watched.json"
        );
        assert_eq!(
            lines[4],
            "preview-sample-event-file=channels/0biwankenobi/videos/XfcLWVX-hCA-arc-raiders-war-tapes-1/event_2026-02-20T18-22-41+00-00_added-to-playlist-favorites.json"
        );
    }

    #[test]
    fn dry_run_preview_reports_when_no_overlap_exists() {
        let playlist_entries = vec![crate::takeout::PlaylistVideoEntry {
            playlist_id: "favorites".to_owned(),
            playlist_name: "Favorites".to_owned(),
            source_file: "favorites.csv".to_owned(),
            video_id: crate::takeout::YouTubeVideoId::new("playlist-only")
                .expect("video id should parse"),
            added_at: DateTime::<FixedOffset>::parse_from_rfc3339("2026-02-20T18:22:41+00:00")
                .expect("playlist timestamp should parse"),
        }];
        let watch_history_entries = vec![crate::takeout::WatchHistoryEntry {
            video_id: crate::takeout::YouTubeVideoId::new("watch-only")
                .expect("video id should parse"),
            title: "Watched Something Else".to_owned(),
            channel_name: Some("Example Channel".to_owned()),
            watched_at: DateTime::<FixedOffset>::parse_from_rfc3339(
                "2025-11-20T23:03:24.580+00:00",
            )
            .expect("watch timestamp should parse"),
        }];

        let lines = build_dry_run_preview_lines(
            Path::new("G:/sync-root"),
            &playlist_entries,
            &watch_history_entries,
        );

        assert_eq!(
            lines,
            vec!["preview-sample=unavailable-no-watch-playlist-overlap"]
        );
    }
}
