use crate::fs_db::ChannelMetadataSnapshotFile;
use crate::fs_db::VideoMetadataSnapshotFile;
use crate::youtube_api::YouTubeFetchedVideoMetadata;
use std::path::Path;
use std::path::PathBuf;

/// Write fetched `YouTube` metadata snapshots into the sync database.
///
/// # Errors
///
/// Returns an error if the snapshot files cannot be created or written.
pub async fn write_fetched_video_metadata(
    sync_dir: &Path,
    fetched_at: &str,
    metadata: &YouTubeFetchedVideoMetadata,
) -> eyre::Result<(PathBuf, PathBuf)> {
    let video_snapshot_path = crate::fs_db::video_snapshot_path_for(
        sync_dir,
        &metadata.channel_name,
        &metadata.title,
        &metadata.video_id,
        fetched_at,
    );
    let channel_snapshot_path =
        crate::fs_db::channel_snapshot_path_for(sync_dir, &metadata.channel_name, fetched_at);

    let video_snapshot = VideoMetadataSnapshotFile {
        fetched_at: fetched_at.to_owned(),
        source_kind: "youtube-data-api-video".to_owned(),
        source_url: metadata.source_url.clone(),
        video_id: metadata.video_id.clone(),
        title: metadata.title.clone(),
        description: metadata.description.clone(),
        channel_id: metadata.channel_id.clone(),
        channel_name: metadata.channel_name.clone(),
        published_at: metadata.published_at.clone(),
        duration_iso8601: metadata.duration_iso8601.clone(),
        view_count: metadata.view_count,
        like_count: metadata.like_count,
        comment_count: metadata.comment_count,
        privacy_status: metadata.privacy_status.clone(),
    };
    let channel_snapshot = ChannelMetadataSnapshotFile {
        fetched_at: fetched_at.to_owned(),
        source_kind: "youtube-data-api-video".to_owned(),
        source_url: metadata.source_url.clone(),
        channel_id: metadata.channel_id.clone(),
        channel_name: metadata.channel_name.clone(),
    };

    write_video_snapshot_file(&video_snapshot_path, &video_snapshot).await?;
    write_channel_snapshot_file(&channel_snapshot_path, &channel_snapshot).await?;

    Ok((video_snapshot_path, channel_snapshot_path))
}

async fn write_video_snapshot_file(
    path: &Path,
    value: &VideoMetadataSnapshotFile,
) -> eyre::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let content = facet_json::to_string_pretty(value)?;
    tokio::fs::write(path, content).await?;
    Ok(())
}

async fn write_channel_snapshot_file(
    path: &Path,
    value: &ChannelMetadataSnapshotFile,
) -> eyre::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let content = facet_json::to_string_pretty(value)?;
    tokio::fs::write(path, content).await?;
    Ok(())
}
