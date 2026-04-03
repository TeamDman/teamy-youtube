use crate::takeout::YouTubeVideoId;
use chrono::DateTime;
use chrono::FixedOffset;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

/// A thumbnail observation stored for a video.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThumbnailObservationRecord {
    pub observed_at: String,
    pub size_key: String,
    pub path: PathBuf,
    pub is_materialized_asset: bool,
}

/// Latest thumbnail observations for a given video, keyed by thumbnail size.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct VideoThumbnailIndex {
    latest_asset_by_size: BTreeMap<String, ThumbnailObservationRecord>,
    latest_observation_by_size: BTreeMap<String, ThumbnailObservationRecord>,
}

impl VideoThumbnailIndex {
    #[must_use]
    pub fn latest_asset_for(&self, size_key: &str) -> Option<&ThumbnailObservationRecord> {
        self.latest_asset_by_size.get(size_key)
    }

    #[must_use]
    pub fn latest_observation_for(&self, size_key: &str) -> Option<&ThumbnailObservationRecord> {
        self.latest_observation_by_size.get(size_key)
    }
}

/// Load the set of video IDs currently present in the sync database.
///
/// # Errors
///
/// Returns an error if the sync directory cannot be read.
pub fn load_video_ids_from_sync_dir(sync_dir: &Path) -> eyre::Result<Vec<YouTubeVideoId>> {
    let videos_dir = sync_dir.join("videos");
    if !videos_dir.exists() {
        return Ok(Vec::new());
    }

    let mut video_ids = Vec::new();
    for entry in std::fs::read_dir(&videos_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let file_name = entry.file_name();
        let Some(video_id) = file_name.to_str() else {
            continue;
        };
        video_ids.push(YouTubeVideoId::new(video_id)?);
    }

    video_ids.sort();
    Ok(video_ids)
}

/// Determine whether a video already has a terminal fetch result in fsdb.
///
/// # Errors
///
/// Returns an error if the video directory cannot be read.
pub fn has_terminal_video_fetch_event(sync_dir: &Path, video_id: &str) -> eyre::Result<bool> {
    let video_dir = crate::fs_db::video_dir_path_for(sync_dir, video_id);
    if !video_dir.exists() {
        return Ok(false);
    }

    for entry in std::fs::read_dir(video_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }

        let Some(file_name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if file_name.starts_with("event_")
            && (file_name.ends_with("_fetch_video_data.json")
                || file_name.ends_with("_fetch_video_data_missing.json")
                || file_name.ends_with("_fetch_video_data_unavailable.json"))
        {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Return the latest successful raw fetch event for a video.
///
/// # Errors
///
/// Returns an error if the video directory cannot be read.
pub fn latest_successful_video_fetch_event_path(
    sync_dir: &Path,
    video_id: &str,
) -> eyre::Result<Option<PathBuf>> {
    let video_dir = crate::fs_db::video_dir_path_for(sync_dir, video_id);
    if !video_dir.exists() {
        return Ok(None);
    }

    let mut latest: Option<(String, PathBuf)> = None;
    for entry in std::fs::read_dir(video_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }

        let Some(file_name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !file_name.starts_with("event_") || !file_name.ends_with("_fetch_video_data.json") {
            continue;
        }

        let path = entry.path();
        match &latest {
            Some((existing_name, _)) if existing_name >= &file_name => {}
            _ => latest = Some((file_name, path)),
        }
    }

    Ok(latest.map(|(_, path)| path))
}

/// Extract the timestamp portion from a successful raw fetch event filename.
///
/// # Errors
///
/// Returns an error if the path does not point at a canonical fetch event file.
pub fn video_fetch_event_timestamp_from_path(fetch_event_path: &Path) -> eyre::Result<String> {
    let file_name = fetch_event_path
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .ok_or_else(|| eyre::eyre!("fetch event path is missing a valid filename"))?;

    file_name
        .strip_prefix("event_")
        .and_then(|value| value.strip_suffix("_fetch_video_data.json"))
        .map(str::to_owned)
        .ok_or_else(|| eyre::eyre!("fetch event path does not use the canonical filename shape"))
}

/// Parse a sanitized event timestamp such as `2026-04-02T15-04-05+00-00`.
///
/// # Errors
///
/// Returns an error if the timestamp does not use the canonical event filename shape.
pub fn parse_sanitized_event_timestamp(value: &str) -> eyre::Result<DateTime<FixedOffset>> {
    let (date_part, time_and_offset) = value
        .split_once('T')
        .ok_or_else(|| eyre::eyre!("event timestamp is missing a time separator"))?;

    if time_and_offset.len() < 6 {
        eyre::bail!("event timestamp is too short to contain a timezone offset");
    }

    let split_index = time_and_offset.len() - 6;
    let (time_part, offset_part) = time_and_offset.split_at(split_index);
    let normalized_time = time_part.replacen('-', ":", 2);
    let normalized_offset = normalize_sanitized_offset(offset_part)?;
    let normalized = format!("{date_part}T{normalized_time}{normalized_offset}");

    DateTime::parse_from_rfc3339(&normalized)
        .map_err(|error| eyre::eyre!("failed parsing event timestamp `{value}`: {error}"))
}

/// Load thumbnail assets and observation events currently present for a video.
///
/// # Errors
///
/// Returns an error if the video directory cannot be read.
pub fn load_video_thumbnail_index(
    sync_dir: &Path,
    video_id: &str,
) -> eyre::Result<VideoThumbnailIndex> {
    let video_dir = crate::fs_db::video_dir_path_for(sync_dir, video_id);
    if !video_dir.exists() {
        return Ok(VideoThumbnailIndex::default());
    }

    let mut index = VideoThumbnailIndex::default();
    for entry in std::fs::read_dir(video_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }

        let Some(file_name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Some(record) = parse_thumbnail_observation_record(&file_name, entry.path()) else {
            continue;
        };

        update_latest_record(&mut index.latest_observation_by_size, record.clone());
        if record.is_materialized_asset {
            update_latest_record(&mut index.latest_asset_by_size, record);
        }
    }

    Ok(index)
}

fn normalize_sanitized_offset(offset_part: &str) -> eyre::Result<String> {
    let (sign, rest) = offset_part.split_at(1);
    if sign != "+" && sign != "-" {
        eyre::bail!("event timestamp offset is missing a leading sign");
    }

    Ok(format!("{sign}{}", rest.replacen('-', ":", 1)))
}

fn parse_thumbnail_observation_record(
    file_name: &str,
    path: PathBuf,
) -> Option<ThumbnailObservationRecord> {
    let rest = file_name.strip_prefix("event_")?;
    let (observed_at, suffix) = rest.split_once("_thumbnail_")?;

    if let Some(size_key) = suffix.strip_suffix("_unchanged.json") {
        return Some(ThumbnailObservationRecord {
            observed_at: observed_at.to_owned(),
            size_key: size_key.to_owned(),
            path,
            is_materialized_asset: false,
        });
    }

    let dot_index = suffix.rfind('.')?;
    let (size_key, _) = suffix.split_at(dot_index);
    Some(ThumbnailObservationRecord {
        observed_at: observed_at.to_owned(),
        size_key: size_key.to_owned(),
        path,
        is_materialized_asset: true,
    })
}

fn update_latest_record(
    map: &mut BTreeMap<String, ThumbnailObservationRecord>,
    record: ThumbnailObservationRecord,
) {
    match map.get(&record.size_key) {
        Some(existing) if existing.observed_at >= record.observed_at => {}
        _ => {
            map.insert(record.size_key.clone(), record);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::has_terminal_video_fetch_event;
    use super::load_video_thumbnail_index;
    use super::parse_sanitized_event_timestamp;
    use super::video_fetch_event_timestamp_from_path;
    use std::path::Path;
    use tempfile::TempDir;

    #[test]
    fn recognizes_negative_fetch_result_files() {
        let temp_dir = TempDir::new().expect("temp dir should be created");
        let path = temp_dir
            .path()
            .join("videos")
            .join("abc123")
            .join("event_2026-01-01T00-00-00+00-00_fetch_video_data_missing.json");
        std::fs::create_dir_all(path.parent().expect("parent should exist"))
            .expect("directories should be created");
        std::fs::write(&path, "{}").expect("file should be written");

        assert!(
            has_terminal_video_fetch_event(temp_dir.path(), "abc123")
                .expect("check should succeed")
        );
    }

    #[test]
    fn extracts_timestamp_from_fetch_event_path() {
        let path = Path::new(
            "G:/sync-root/videos/abc123/event_2026-04-02T15-04-05+00-00_fetch_video_data.json",
        );

        assert_eq!(
            video_fetch_event_timestamp_from_path(path).expect("timestamp should be parsed"),
            "2026-04-02T15-04-05+00-00"
        );
    }

    #[test]
    fn parses_sanitized_event_timestamp() {
        let parsed = parse_sanitized_event_timestamp("2026-04-02T15-04-05+00-00")
            .expect("timestamp should parse");

        assert_eq!(parsed.to_rfc3339(), "2026-04-02T15:04:05+00:00");
    }

    #[test]
    fn loads_latest_thumbnail_assets_and_observations_by_size() {
        let temp_dir = TempDir::new().expect("temp dir should be created");
        let video_dir = temp_dir.path().join("videos").join("abc123");
        std::fs::create_dir_all(&video_dir).expect("video directory should be created");

        std::fs::write(
            video_dir.join("event_2026-04-02T15-04-05+00-00_thumbnail_120x90.jpg"),
            b"old",
        )
        .expect("old asset should be written");
        std::fs::write(
            video_dir.join("event_2026-04-02T16-04-05+00-00_thumbnail_120x90_unchanged.json"),
            "{}",
        )
        .expect("unchanged event should be written");
        std::fs::write(
            video_dir.join("event_2026-04-02T17-04-05+00-00_thumbnail_120x90.jpg"),
            b"new",
        )
        .expect("new asset should be written");
        std::fs::write(
            video_dir.join("event_2026-04-02T18-04-05+00-00_thumbnail_120x90_unchanged.json"),
            "{}",
        )
        .expect("newest unchanged event should be written");

        let index = load_video_thumbnail_index(temp_dir.path(), "abc123")
            .expect("thumbnail index should load");

        assert_eq!(
            index
                .latest_asset_for("120x90")
                .expect("asset should exist")
                .observed_at,
            "2026-04-02T17-04-05+00-00"
        );
        assert_eq!(
            index
                .latest_observation_for("120x90")
                .expect("observation should exist")
                .observed_at,
            "2026-04-02T18-04-05+00-00"
        );
    }
}
