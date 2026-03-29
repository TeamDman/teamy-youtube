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
pub struct SyncNowTakeoutArgs {
    /// Print a summary without writing the sync database.
    #[facet(args::named, default)]
    pub dry_run: bool,

    /// Optional directory to scan instead of discovering the latest takeout via teamy-mft.
    #[facet(args::named)]
    pub input_dir: Option<String>,
}

impl SyncNowTakeoutArgs {
    /// # Errors
    ///
    /// This function will return an error if the sync dir is unset, discovery fails,
    /// the takeout inputs cannot be read, or the sync database cannot be written.
    pub async fn invoke(self) -> eyre::Result<()> {
        let sync_dir = crate::paths::try_get_sync_dir()?;
        let (watch_history_json, playlist_csvs, source_label) =
            if let Some(input_dir) = self.input_dir {
                let input_dir = normalize_input_dir(&input_dir)?;
                let discovered = discover_takeout_inputs_from_directory(&input_dir)?;
                (discovered.0, discovered.1, input_dir.display().to_string())
            } else {
                let discovered = discover_takeout_inputs_from_index()?;
                (discovered.0, discovered.1, "teamy-mft-index".to_owned())
            };

        info!(
            sync_dir = %sync_dir.display(),
            watch_history_json = %watch_history_json.display(),
            playlist_csv_count = playlist_csvs.len(),
            dry_run = self.dry_run,
            "syncing from takeout inputs"
        );

        let mut playlist_entries = Vec::new();
        for playlist_csv in &playlist_csvs {
            let playlist_name = playlist_name_from_csv_path(playlist_csv)?;
            let mut entries =
                crate::takeout::read_playlist_video_entries(playlist_csv, playlist_name).await?;
            playlist_entries.append(&mut entries);
        }

        let watch_history_report =
            crate::takeout::read_watch_history_entries(&watch_history_json).await?;
        let imported_at = Local::now().to_rfc3339();
        let sync_summary = crate::fs_db::write_takeout_sync(
            &sync_dir,
            &imported_at,
            &watch_history_json,
            &playlist_entries,
            &watch_history_report.entries,
            self.dry_run,
        )
        .await?;

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

        println!("source={source_label}");
        println!("sync-dir={}", sync_dir.display());
        println!("dry-run={}", self.dry_run);
        println!("watch-history-json={}", watch_history_json.display());
        println!("playlist-csv-count={}", playlist_csvs.len());
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

        Ok(())
    }
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
