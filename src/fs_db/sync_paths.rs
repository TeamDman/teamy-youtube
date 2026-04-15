use std::path::Path;
use std::path::PathBuf;

/// Build the canonical path for a video event in the sync database.
#[must_use]
pub fn event_path_for(
    sync_dir: &Path,
    _channel_name: Option<&str>,
    _video_title: Option<&str>,
    video_id: &str,
    event_at: &str,
    event_suffix: &str,
) -> PathBuf {
    video_dir_path_for(sync_dir, video_id).join(format!(
        "event_{}_{}.json",
        sanitize_timestamp(event_at),
        event_suffix
    ))
}

/// Build the canonical path for a raw fetched video-data event.
#[must_use]
pub fn video_fetch_event_path_for(sync_dir: &Path, video_id: &str, fetched_at: &str) -> PathBuf {
    video_dir_path_for(sync_dir, video_id).join(format!(
        "event_{}_fetch_video_data.json",
        sanitize_timestamp(fetched_at)
    ))
}

/// Build the canonical path for a title observation text file.
#[must_use]
pub fn video_title_observation_path_for(
    sync_dir: &Path,
    video_id: &str,
    observed_at: &str,
    title: &str,
) -> PathBuf {
    video_dir_path_for(sync_dir, video_id).join(format!(
        "event_{}_observe_title_{}.txt",
        sanitize_timestamp(observed_at),
        sanitize_component(&normalize_title_for_path(title))
    ))
}

// yt[storage.video-thumbnail.layout]
/// Build the canonical path for a downloaded thumbnail asset.
#[must_use]
pub fn video_thumbnail_path_for(
    sync_dir: &Path,
    video_id: &str,
    observed_at: &str,
    thumbnail_size: &str,
    source_url: &str,
) -> PathBuf {
    let extension = thumbnail_extension_from_url(source_url);
    video_dir_path_for(sync_dir, video_id).join(format!(
        "event_{}_thumbnail_{}.{}",
        sanitize_timestamp(observed_at),
        sanitize_component(thumbnail_size),
        extension
    ))
}

// yt[storage.video-thumbnail.unchanged-layout]
/// Build the canonical path for an unchanged-thumbnail observation event.
#[must_use]
pub fn video_thumbnail_unchanged_event_path_for(
    sync_dir: &Path,
    video_id: &str,
    observed_at: &str,
    thumbnail_size: &str,
) -> PathBuf {
    video_dir_path_for(sync_dir, video_id).join(format!(
        "event_{}_thumbnail_{}_unchanged.json",
        sanitize_timestamp(observed_at),
        sanitize_component(thumbnail_size)
    ))
}

/// Build the canonical path for an unavailable-thumbnail observation event.
#[must_use]
pub fn video_thumbnail_unavailable_event_path_for(
    sync_dir: &Path,
    video_id: &str,
    observed_at: &str,
    thumbnail_size: &str,
) -> PathBuf {
    video_dir_path_for(sync_dir, video_id).join(format!(
        "event_{}_thumbnail_{}_unavailable.json",
        sanitize_timestamp(observed_at),
        sanitize_component(thumbnail_size)
    ))
}

/// Build the canonical directory path for a video in the sync database.
#[must_use]
pub fn video_dir_path_for(sync_dir: &Path, video_id: &str) -> PathBuf {
    sync_dir.join("videos").join(video_id)
}

/// Build the canonical event-id suffix for a playlist-membership event.
#[must_use]
pub fn playlist_event_suffix(playlist_id: &str) -> String {
    format!("added-to-playlist-{}", sanitize_component(playlist_id))
}

fn sanitize_timestamp(value: &str) -> String {
    value.replace(':', "-")
}

fn normalize_title_for_path(value: &str) -> String {
    let trimmed = value.trim();
    if let Some(stripped) = strip_ascii_prefix_ignore_case(trimmed, "watched ") {
        return stripped.trim().to_owned();
    }
    if let Some(stripped) = strip_ascii_prefix_ignore_case(trimmed, "watched-") {
        return stripped.trim().to_owned();
    }

    trimmed.to_owned()
}

fn strip_ascii_prefix_ignore_case<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    let mut remainder = value;
    for expected in prefix.chars() {
        let mut chars = remainder.chars();
        let actual = chars.next()?;
        if !actual.eq_ignore_ascii_case(&expected) {
            return None;
        }
        remainder = chars.as_str();
    }

    Some(remainder)
}

fn sanitize_component(value: &str) -> String {
    let mut sanitized = String::new();
    let mut previous_was_dash = false;

    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            sanitized.push(character.to_ascii_lowercase());
            previous_was_dash = false;
        } else if !previous_was_dash {
            sanitized.push('-');
            previous_was_dash = true;
        }
    }

    let sanitized = sanitized.trim_matches('-');
    if sanitized.is_empty() {
        "unknown".to_owned()
    } else {
        sanitized.to_owned()
    }
}

fn thumbnail_extension_from_url(source_url: &str) -> String {
    let extension = reqwest::Url::parse(source_url)
        .ok()
        .and_then(|url| {
            url.path_segments()
                .and_then(|mut segments| segments.next_back().map(str::to_owned))
        })
        .and_then(|file_name| {
            Path::new(&file_name)
                .extension()
                .and_then(std::ffi::OsStr::to_str)
                .map(str::to_ascii_lowercase)
        });

    extension.unwrap_or_else(|| "bin".to_owned())
}

#[cfg(test)]
mod tests {
    use super::normalize_title_for_path;
    use super::video_thumbnail_path_for;
    use super::video_thumbnail_unavailable_event_path_for;
    use super::video_thumbnail_unchanged_event_path_for;
    use std::path::Path;

    #[test]
    fn builds_thumbnail_path_from_url_extension() {
        let path = video_thumbnail_path_for(
            Path::new("G:/sync-root"),
            "abc123",
            "2026-04-02T15:04:05+00:00",
            "1280x720",
            "https://example.invalid/path/image.webp?foo=bar",
        );

        assert_eq!(
            path.display().to_string().replace('\\', "/"),
            "G:/sync-root/videos/abc123/event_2026-04-02T15-04-05+00-00_thumbnail_1280x720.webp"
        );
    }

    #[test]
    fn builds_unchanged_thumbnail_event_path() {
        let path = video_thumbnail_unchanged_event_path_for(
            Path::new("G:/sync-root"),
            "abc123",
            "2026-04-02T15:04:05+00:00",
            "120x90",
        );

        assert_eq!(
            path.display().to_string().replace('\\', "/"),
            "G:/sync-root/videos/abc123/event_2026-04-02T15-04-05+00-00_thumbnail_120x90_unchanged.json"
        );
    }

    #[test]
    fn builds_unavailable_thumbnail_event_path() {
        let path = video_thumbnail_unavailable_event_path_for(
            Path::new("G:/sync-root"),
            "abc123",
            "2026-04-02T15:04:05+00:00",
            "120x90",
        );

        assert_eq!(
            path.display().to_string().replace('\\', "/"),
            "G:/sync-root/videos/abc123/event_2026-04-02T15-04-05+00-00_thumbnail_120x90_unavailable.json"
        );
    }

    #[test]
    fn normalizes_ascii_watched_prefix_for_unicode_title() {
        assert_eq!(
            normalize_title_for_path("Watched 【東方ヴォーカルPV】LOVE EAST【暁Records公式】"),
            "【東方ヴォーカルPV】LOVE EAST【暁Records公式】"
        );
    }

    #[test]
    fn leaves_unicode_title_without_prefix_unchanged() {
        assert_eq!(
            normalize_title_for_path("【東方ヴォーカルPV】LOVE EAST【暁Records公式】"),
            "【東方ヴォーカルPV】LOVE EAST【暁Records公式】"
        );
    }
}
