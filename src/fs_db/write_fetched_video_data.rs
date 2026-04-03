use crate::youtube_api::YouTubeFetchedVideoMetadata;
use std::path::Path;
use std::path::PathBuf;
use tracing::debug;

/// Write raw fetched `YouTube` video data into the sync database.
///
/// # Errors
///
/// Returns an error if the event or observation files cannot be created or written.
pub async fn write_fetched_video_data(
    sync_dir: &Path,
    fetched_at: &str,
    metadata: &YouTubeFetchedVideoMetadata,
) -> eyre::Result<(PathBuf, PathBuf)> {
    let raw_response_bytes = metadata.raw_response_body.as_str().len();
    let title_bytes = metadata.title.len();
    let fetch_event_path =
        crate::fs_db::video_fetch_event_path_for(sync_dir, &metadata.video_id, fetched_at);
    let title_observation_path = crate::fs_db::video_title_observation_path_for(
        sync_dir,
        &metadata.video_id,
        fetched_at,
        &metadata.title,
    );

    if let Some(parent) = fetch_event_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    debug!(
        video_id = %metadata.video_id,
        fetch_event_path = %fetch_event_path.display(),
        title_observation_path = %title_observation_path.display(),
        raw_response_bytes,
        raw_response_bytes_human = %crate::sync_progress::format_bytes(u64::try_from(raw_response_bytes)?),
        title_bytes,
        title_bytes_human = %crate::sync_progress::format_bytes(u64::try_from(title_bytes)?),
        "writing fetched video data to disk"
    );

    tokio::fs::write(&fetch_event_path, metadata.raw_response_body.as_str()).await?;
    tokio::fs::write(&title_observation_path, &metadata.title).await?;

    Ok((fetch_event_path, title_observation_path))
}
