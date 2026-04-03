use facet_json::RawJson;
use std::path::Path;
use std::path::PathBuf;
use tracing::debug;

/// Write a negative video-fetch result into the sync database.
///
/// # Errors
///
/// Returns an error if the event file cannot be created or written.
pub async fn write_missing_video_data(
    sync_dir: &Path,
    fetched_at: &str,
    video_id: &str,
    event_suffix: &str,
    raw_response_body: &RawJson<'_>,
) -> eyre::Result<PathBuf> {
    let raw_response_bytes = raw_response_body.as_str().len();
    let event_path =
        crate::fs_db::event_path_for(sync_dir, None, None, video_id, fetched_at, event_suffix);
    if let Some(parent) = event_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    debug!(
        video_id,
        event_suffix,
        event_path = %event_path.display(),
        raw_response_bytes,
        raw_response_bytes_human = %crate::sync_progress::format_bytes(u64::try_from(raw_response_bytes)?),
        "writing negative fetch result to disk"
    );

    tokio::fs::write(&event_path, raw_response_body.as_str()).await?;
    Ok(event_path)
}
