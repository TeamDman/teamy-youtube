use super::ArchiveEntryInventory;
use super::ArchiveInspection;
use super::ArchiveReadLimits;
use super::ArchiveReadRequest;
use super::ArchiveReadResult;
use super::ArchiveSnapshot;
use super::ArchiveWatchRecord;
use super::SelectedArchive;
use crate::takeout::WatchHistoryEntry;
use crate::takeout::raw_watch_history_entry::RawWatchHistoryEntry;
use eyre::Context;
use std::collections::BTreeSet;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::UNIX_EPOCH;
use zip::ZipArchive;

/// Capture filesystem identity without reading archive contents.
/// # Errors
/// Returns an error for inaccessible files or unsupported timestamps.
pub fn snapshot_archive(path: &Path) -> eyre::Result<ArchiveSnapshot> {
    let path = dunce::canonicalize(path).wrap_err("Cannot resolve selected ZIP path")?;
    let metadata = std::fs::metadata(&path)?;
    eyre::ensure!(metadata.is_file(), "Selected ZIP must be a file");
    Ok(ArchiveSnapshot {
        path: path.display().to_string(),
        bytes: metadata.len(),
        modified_unix_ms: u64::try_from(
            metadata.modified()?.duration_since(UNIX_EPOCH)?.as_millis(),
        )?,
    })
}

/// Inventory central-directory metadata only. No entries are extracted or decoded.
/// # Errors
/// Returns an error for changed, malformed, unsafe, ambiguous or over-limit archives.
pub fn inspect_archive(
    selected: SelectedArchive,
    limits: &ArchiveReadLimits,
) -> eyre::Result<ArchiveInspection> {
    eyre::ensure!(
        limits.max_entries > 0 && limits.max_entry_bytes > 0,
        "Archive limits must be positive"
    );
    ensure_snapshot(&selected.snapshot)?;
    let file = File::open(&selected.snapshot.path)?;
    let mut archive = ZipArchive::new(file).wrap_err("Malformed ZIP central directory")?;
    eyre::ensure!(
        archive.len() <= limits.max_entries,
        "ZIP entry-count limit exceeded"
    );
    validate_central_directory(
        &selected.snapshot.path,
        archive.central_directory_start(),
        limits.max_entries,
    )?;
    let mut names = BTreeSet::new();
    let mut entries = Vec::with_capacity(archive.len());
    let mut history_entry = None;
    for index in 0..archive.len() {
        let entry = archive
            .by_index_raw(index)
            .wrap_err_with(|| format!("Cannot inspect ZIP entry {index}"))?;
        let name = normalized_entry_name(entry.name())?;
        eyre::ensure!(
            names.insert(name.clone()),
            "Ambiguous duplicate ZIP entry name at index {index}"
        );
        let kind = if entry.is_dir() {
            "directory"
        } else if is_youtube_section(&name, "history/watch-history.json") {
            eyre::ensure!(
                history_entry.is_none(),
                "Ambiguous multiple YouTube watch-history entries"
            );
            eyre::ensure!(
                entry.size() <= limits.max_entry_bytes,
                "Watch-history entry exceeds byte limit"
            );
            eyre::ensure!(
                !entry.is_symlink(),
                "Watch-history entry cannot be a symbolic link"
            );
            history_entry = Some(name.clone());
            "watch-history-json"
        } else if is_youtube_section(&name, "history/watch-history.html") {
            "watch-history-html-unsupported"
        } else if is_youtube_playlist(&name) {
            "playlist-csv"
        } else {
            "other"
        };
        entries.push(ArchiveEntryInventory {
            name,
            kind: kind.to_owned(),
            bytes: entry.size(),
            compressed_bytes: entry.compressed_size(),
        });
    }
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    ensure_snapshot(&selected.snapshot)?;
    Ok(ArchiveInspection {
        selected,
        entries,
        history_entry,
    })
}

/// Decode only the recognized watch-history entry using a bounded buffer.
/// This is entry-buffered JSON parsing, not record-by-record streaming.
/// # Errors
/// Returns an error for missing JSON history, invalid data or resource-limit violations.
pub fn read_archive(request: ArchiveReadRequest) -> eyre::Result<ArchiveReadResult> {
    let inspection = inspect_archive(request.selected, &request.limits)?;
    let entry_name = inspection.history_entry.as_ref().ok_or_else(|| {
        eyre::eyre!(
            "ZIP has no supported YouTube history/watch-history.json; HTML history is not supported"
        )
    })?;
    let mut archive = ZipArchive::new(File::open(&inspection.selected.snapshot.path)?)?;
    let index = (0..archive.len())
        .find(|&index| {
            archive
                .name_for_index(index)
                .is_some_and(|name| name.replace('\\', "/") == *entry_name)
        })
        .ok_or_else(|| eyre::eyre!("Selected history entry disappeared"))?;
    let entry = archive
        .by_index(index)
        .wrap_err("Cannot open selected history entry")?;
    let mut bytes = Vec::new();
    entry
        .take(
            request
                .limits
                .max_entry_bytes
                .checked_add(1)
                .ok_or_else(|| eyre::eyre!("Byte limit is too large"))?,
        )
        .read_to_end(&mut bytes)
        .wrap_err("Cannot read selected history entry (including ZIP checksum validation)")?;
    eyre::ensure!(
        u64::try_from(bytes.len())? <= request.limits.max_entry_bytes,
        "Decoded history exceeds byte limit"
    );
    let json = std::str::from_utf8(&bytes).wrap_err("History entry is not UTF-8")?;
    let raw_entries: Vec<RawWatchHistoryEntry> = facet_json::from_str(json)
        .map_err(|_error| eyre::eyre!("Malformed watch-history JSON (record contents omitted)"))?;
    let mut records = Vec::new();
    let mut skipped_record_count = 0;
    for (record_index, raw) in raw_entries.into_iter().enumerate() {
        let channel_url = raw
            .subtitles
            .first()
            .and_then(|subtitle| subtitle.url.clone());
        let channel_id = channel_url.as_deref().and_then(channel_id_from_url);
        let record = WatchHistoryEntry::try_from_raw(raw).map_err(|_error| {
            eyre::eyre!("Malformed watch-history record {record_index} (record contents omitted)")
        })?;
        if let Some(record) = record {
            records.push(ArchiveWatchRecord {
                video_id: record.video_id.to_string(),
                watched_at: record.watched_at.to_rfc3339(),
                title: record.title,
                channel_name: record.channel_name,
                channel_url,
                channel_id,
                archive_path: inspection.selected.snapshot.path.clone(),
                entry_name: entry_name.clone(),
                record_index,
            });
        } else {
            skipped_record_count += 1;
        }
    }
    ensure_snapshot(&inspection.selected.snapshot)?;
    Ok(ArchiveReadResult {
        inspection,
        parsing_mode: "bounded-entry-buffer; JSON array decoded in memory; no extraction"
            .to_owned(),
        skipped_record_count,
        records,
    })
}

fn ensure_snapshot(snapshot: &ArchiveSnapshot) -> eyre::Result<()> {
    eyre::ensure!(
        *snapshot == snapshot_archive(Path::new(&snapshot.path))?,
        "ZIP changed since selection; select it again"
    );
    Ok(())
}

fn normalized_entry_name(name: &str) -> eyre::Result<String> {
    let name = name.replace('\\', "/");
    eyre::ensure!(
        !name.is_empty() && !name.starts_with('/') && !name.contains(':') && !name.contains('\0'),
        "Unsafe ZIP entry path"
    );
    eyre::ensure!(
        name.trim_end_matches('/')
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != ".."),
        "Unsafe ZIP entry path"
    );
    Ok(name)
}

fn is_youtube_section(name: &str, suffix: &str) -> bool {
    ["YouTube and YouTube Music", "YouTube"]
        .iter()
        .any(|product| {
            let section = format!("{product}/{suffix}");
            name == section || name.ends_with(&format!("/{section}"))
        })
}

fn is_youtube_playlist(name: &str) -> bool {
    name.ends_with("-videos.csv")
        && ["YouTube and YouTube Music", "YouTube"]
            .iter()
            .any(|product| name.contains(&format!("{product}/playlists/")))
}

fn channel_id_from_url(value: &str) -> Option<String> {
    let url = reqwest::Url::parse(value).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || !matches!(
            url.host_str(),
            Some("www.youtube.com" | "youtube.com" | "music.youtube.com")
        )
    {
        return None;
    }
    let id = url.path().strip_prefix("/channel/")?;
    (id.len() == 24
        && id.starts_with("UC")
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')))
    .then(|| id.to_owned())
}

// ZIP's name map overwrites exact duplicate names. Check the original central
// headers independently so ambiguity cannot disappear before inventory validation.
fn validate_central_directory(path: &str, offset: u64, max_entries: usize) -> eyre::Result<()> {
    use std::io::Seek;
    use std::io::SeekFrom;

    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(offset))?;
    let mut names = BTreeSet::new();
    let mut count = 0usize;
    loop {
        let mut header = [0u8; 46];
        file.read_exact(&mut header[..4])
            .wrap_err("Truncated ZIP central directory")?;
        let signature = u32::from_le_bytes(header[..4].try_into()?);
        if matches!(
            signature,
            0x0605_4b50 | 0x0606_4b50 | 0x0706_4b50 | 0x0505_4b50
        ) {
            break;
        }
        eyre::ensure!(
            signature == 0x0201_4b50,
            "Malformed ZIP central directory header"
        );
        count += 1;
        eyre::ensure!(count <= max_entries, "ZIP entry-count limit exceeded");
        file.read_exact(&mut header[4..])?;
        let name_len = u16::from_le_bytes([header[28], header[29]]);
        let extra_len = u16::from_le_bytes([header[30], header[31]]);
        let comment_len = u16::from_le_bytes([header[32], header[33]]);
        let mut name = vec![0; usize::from(name_len)];
        file.read_exact(&mut name)?;
        eyre::ensure!(
            names.insert(name),
            "Ambiguous duplicate ZIP central-directory name"
        );
        file.seek(SeekFrom::Current(
            i64::from(extra_len) + i64::from(comment_len),
        ))?;
    }
    Ok(())
}
