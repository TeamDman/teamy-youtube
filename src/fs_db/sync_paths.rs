use std::path::Path;
use std::path::PathBuf;

/// Build the canonical path for a video event in the sync database.
#[must_use]
pub fn event_path_for(
    sync_dir: &Path,
    channel_name: Option<&str>,
    video_title: Option<&str>,
    video_id: &str,
    event_at: &str,
    event_suffix: &str,
) -> PathBuf {
    video_dir_path_for(sync_dir, channel_name, video_title, video_id).join(format!(
        "event_{}_{}.json",
        sanitize_timestamp(event_at),
        event_suffix
    ))
}

/// Build the canonical path for a video metadata snapshot in the sync database.
#[must_use]
pub fn video_snapshot_path_for(
    sync_dir: &Path,
    channel_name: &str,
    video_title: &str,
    video_id: &str,
    fetched_at: &str,
) -> PathBuf {
    video_dir_path_for(sync_dir, Some(channel_name), Some(video_title), video_id).join(format!(
        "snapshot_{}_video.json",
        sanitize_timestamp(fetched_at)
    ))
}

/// Build the canonical path for a channel metadata snapshot in the sync database.
#[must_use]
pub fn channel_snapshot_path_for(sync_dir: &Path, channel_name: &str, fetched_at: &str) -> PathBuf {
    sync_dir
        .join("channels")
        .join(sanitize_component(channel_name))
        .join(format!(
            "snapshot_{}_channel.json",
            sanitize_timestamp(fetched_at)
        ))
}

/// Build the canonical event-id suffix for a playlist-membership event.
#[must_use]
pub fn playlist_event_suffix(playlist_id: &str) -> String {
    format!("added-to-playlist-{}", sanitize_component(playlist_id))
}

fn video_dir_path_for(
    sync_dir: &Path,
    channel_name: Option<&str>,
    video_title: Option<&str>,
    video_id: &str,
) -> PathBuf {
    let channel_slug = sanitize_component(channel_name.unwrap_or("unknown-channel"));
    let title_slug = video_title
        .map(normalize_title_for_path)
        .map_or_else(|| video_id.to_owned(), |value| sanitize_component(&value));
    let video_slug = if title_slug == video_id {
        title_slug
    } else {
        format!("{video_id}-{title_slug}")
    };

    sync_dir
        .join("channels")
        .join(channel_slug)
        .join("videos")
        .join(video_slug)
}

fn sanitize_timestamp(value: &str) -> String {
    value.replace(':', "-")
}

fn normalize_title_for_path(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() > "watched ".len()
        && trimmed[.."watched ".len()].eq_ignore_ascii_case("watched ")
    {
        return trimmed["watched ".len()..].trim().to_owned();
    }
    if trimmed.len() > "watched-".len()
        && trimmed[.."watched-".len()].eq_ignore_ascii_case("watched-")
    {
        return trimmed["watched-".len()..].trim().to_owned();
    }

    trimmed.to_owned()
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
