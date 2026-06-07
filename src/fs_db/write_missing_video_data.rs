use eyre::Context;
use eyre::eyre;
use facet_json::RawJson;
use std::path::Path;
use std::path::PathBuf;
use tracing::debug;

use crate::takeout::YouTubeVideoId;

/// Write a negative video-fetch result into the sync database.
///
/// # Errors
///
/// Returns an error if the event file cannot be created or written.
pub async fn write_missing_video_data(
    sync_dir: &Path,
    fetched_at: &str,
    video_id: YouTubeVideoId,
    event_suffix: &str,
    raw_response_body: &RawJson<'_>,
) -> eyre::Result<PathBuf> {
    let raw_response_bytes = raw_response_body.as_str().len();
    let event_path =
        crate::fs_db::event_path_for(sync_dir, None, None, video_id, fetched_at, event_suffix);
    if let Some(parent) = event_path.parent() {
        tokio::fs::create_dir_all(parent).await.wrap_err_with(|| {
            eyre!(
                "Failed to create parent directory '{}' for '{}'",
                parent.display(),
                event_path.display()
            )
        })?;
    }

    debug!(
        video_id = %video_id,
        event_suffix,
        event_path = %event_path.display(),
        raw_response_bytes,
        raw_response_bytes_human = %crate::sync_progress::format_bytes(u64::try_from(raw_response_bytes)?),
        "writing negative fetch result to disk"
    );

    tokio::fs::write(&event_path, raw_response_body.as_str()).await?;
    Ok(event_path)
}

#[cfg(test)]
mod tests {
    use facet_json::RawJson;

    #[tokio::test]
    async fn url_input_writes_missing_event_under_canonical_video_id_directory() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let video = "https://www.youtube.com/watch?v=tCUBfHz3UUA"
            .parse::<crate::takeout::YouTubeVideoArgument<'static>>()
            .expect("video argument should parse");
        let path = super::write_missing_video_data(
            temp_dir.path(),
            "2026-06-07T16:17:05.523601400-04:00",
            video.video_id(),
            "fetch_video_data_missing",
            &RawJson::from("{}"),
        )
        .await
        .expect("missing event should be written");

        let normalized = path.display().to_string().replace('\\', "/");
        assert!(normalized.contains("/videos/tCUBfHz3UUA/"));
        assert!(!normalized.contains("https://www.youtube.com/watch?v=tCUBfHz3UUA"));
    }
}
