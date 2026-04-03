mod playlist_video_entry;
mod raw_watch_history_entry;
mod raw_watch_history_subtitle;
mod watch_history_entry;
mod youtube_video_id;

use eyre::WrapErr as _;
pub use playlist_video_entry::*;
use std::path::Path;
use tracing::debug;
use tracing::info;
pub use watch_history_entry::*;
pub use youtube_video_id::*;

const WATCH_LATER_HEADER: &str = "Video ID,Playlist Video Creation Timestamp";

/// Read a generic playlist CSV from Google Takeout.
///
/// # Errors
///
/// Returns an error if the file cannot be read or if any non-header row is malformed.
pub async fn read_playlist_video_entries(
    path: &Path,
    playlist_name: String,
) -> eyre::Result<Vec<PlaylistVideoEntry>> {
    let csv = tokio::fs::read_to_string(path)
        .await
        .wrap_err_with(|| format!("failed to read playlist CSV from {}", path.display()))?;

    let mut lines = csv.lines();
    let Some(header) = lines.next() else {
        eyre::bail!("playlist CSV at {} is empty", path.display());
    };

    if header.trim() != WATCH_LATER_HEADER {
        eyre::bail!(
            "unexpected playlist CSV header in {}: expected {WATCH_LATER_HEADER:?}, found {:?}",
            path.display(),
            header,
        );
    }

    let playlist_id = slugify_playlist_name(&playlist_name);
    let mut entries = Vec::new();
    for (index, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let line_number = index + 2;
        let entry = PlaylistVideoEntry::parse_csv_line(
            line_number,
            line,
            playlist_id.clone(),
            playlist_name.clone(),
            path.display().to_string(),
        )
        .wrap_err_with(|| {
            format!(
                "failed to parse playlist CSV row {line_number} from {}",
                path.display()
            )
        })?;
        entries.push(entry);
    }

    info!(
        path = %path.display(),
        playlist_name,
        entry_count = entries.len(),
        "loaded playlist entries"
    );
    Ok(entries)
}

fn slugify_playlist_name(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_was_dash = false;

    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            previous_was_dash = false;
        } else if !previous_was_dash {
            slug.push('-');
            previous_was_dash = true;
        }
    }

    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "playlist".to_owned()
    } else {
        slug.to_owned()
    }
}

/// Parsed watch-history entries plus skip statistics for unsupported rows.
#[derive(Debug, PartialEq)]
pub struct WatchHistoryReport {
    pub entries: Vec<WatchHistoryEntry>,
    pub skipped_entry_count: usize,
}

/// Read the watch-history JSON from Google Takeout.
///
/// # Errors
///
/// Returns an error if the file cannot be read or if any supported row is malformed.
pub async fn read_watch_history_entries(path: &Path) -> eyre::Result<WatchHistoryReport> {
    let json = tokio::fs::read_to_string(path)
        .await
        .wrap_err_with(|| format!("failed to read watch-history JSON from {}", path.display()))?;
    let raw_entries: Vec<raw_watch_history_entry::RawWatchHistoryEntry> =
        facet_json::from_str(&json).wrap_err_with(|| {
            format!("failed to parse watch-history JSON from {}", path.display())
        })?;

    let mut entries = Vec::new();
    let mut skipped_entry_count = 0;

    for (index, raw_entry) in raw_entries.into_iter().enumerate() {
        match WatchHistoryEntry::try_from_raw(raw_entry).wrap_err_with(|| {
            format!(
                "failed to parse watch-history entry {} from {}",
                index + 1,
                path.display()
            )
        })? {
            Some(entry) => entries.push(entry),
            None => skipped_entry_count += 1,
        }
    }

    debug!(
        path = %path.display(),
        parsed_entry_count = entries.len(),
        skipped_entry_count,
        "loaded watch-history entries"
    );
    Ok(WatchHistoryReport {
        entries,
        skipped_entry_count,
    })
}
