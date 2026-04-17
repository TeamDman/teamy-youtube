use crate::channel_sync::ChannelDiscoveryEventFile;
use crate::channel_sync::ChannelSyncTargetFile;
use crate::channel_sync::VideoDownloadCompletedEventFile;
use crate::channel_sync::VideoDownloadFailedEventFile;
use crate::channel_sync::VideoDownloadRequestEventFile;
use eyre::WrapErr as _;
use std::path::Path;
use std::path::PathBuf;

const CHANNEL_TARGET_FILE_NAME: &str = "target.json";

pub fn channel_target_path_for(sync_dir: &Path, channel_id: &str) -> PathBuf {
    channel_dir_path_for(sync_dir, channel_id).join(CHANNEL_TARGET_FILE_NAME)
}

pub async fn write_channel_sync_target(
    sync_dir: &Path,
    target: &ChannelSyncTargetFile,
) -> eyre::Result<PathBuf> {
    let path = channel_target_path_for(sync_dir, &target.channel_id);
    write_json_file(&path, target).await
}

pub async fn write_channel_discovery_event(
    sync_dir: &Path,
    event_file: &ChannelDiscoveryEventFile,
) -> eyre::Result<PathBuf> {
    let path = channel_discovery_event_path_for(
        sync_dir,
        &event_file.channel_id,
        &event_file.discovered_at,
    );
    write_json_file(&path, event_file).await
}

pub async fn write_video_download_request_event(
    sync_dir: &Path,
    event_file: &VideoDownloadRequestEventFile,
) -> eyre::Result<PathBuf> {
    let path = crate::fs_db::video_download_request_event_path_for(
        sync_dir,
        &event_file.video_id,
        &event_file.requested_at,
    );
    write_json_file(&path, event_file).await
}

pub async fn write_video_download_completed_event(
    sync_dir: &Path,
    event_file: &VideoDownloadCompletedEventFile,
) -> eyre::Result<PathBuf> {
    let path = crate::fs_db::video_download_completed_event_path_for(
        sync_dir,
        &event_file.video_id,
        &event_file.downloaded_at,
    );
    write_json_file(&path, event_file).await
}

pub async fn write_video_download_failed_event(
    sync_dir: &Path,
    event_file: &VideoDownloadFailedEventFile,
) -> eyre::Result<PathBuf> {
    let path = crate::fs_db::video_download_failed_event_path_for(
        sync_dir,
        &event_file.video_id,
        &event_file.failed_at,
    );
    write_json_file(&path, event_file).await
}

pub fn load_channel_sync_targets(sync_dir: &Path) -> eyre::Result<Vec<ChannelSyncTargetFile>> {
    let channels_dir = sync_dir.join("channels");
    if !channels_dir.exists() {
        return Ok(Vec::new());
    }

    let mut targets = Vec::new();
    for entry in std::fs::read_dir(channels_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let target_path = entry.path().join(CHANNEL_TARGET_FILE_NAME);
        if !target_path.is_file() {
            continue;
        }

        let content = std::fs::read_to_string(&target_path)
            .wrap_err_with(|| format!("failed reading {}", target_path.display()))?;
        let target: ChannelSyncTargetFile = facet_json::from_str(&content)
            .wrap_err_with(|| format!("failed parsing {}", target_path.display()))?;
        targets.push(target);
    }

    targets.sort_by(|left, right| left.channel_id.cmp(&right.channel_id));
    Ok(targets)
}

pub fn read_video_download_request_event(
    path: &Path,
) -> eyre::Result<VideoDownloadRequestEventFile> {
    let content = std::fs::read_to_string(path)
        .wrap_err_with(|| format!("failed reading {}", path.display()))?;
    facet_json::from_str(&content).wrap_err_with(|| format!("failed parsing {}", path.display()))
}

fn channel_dir_path_for(sync_dir: &Path, channel_id: &str) -> PathBuf {
    sync_dir.join("channels").join(channel_id)
}

fn channel_discovery_event_path_for(
    sync_dir: &Path,
    channel_id: &str,
    discovered_at: &str,
) -> PathBuf {
    channel_dir_path_for(sync_dir, channel_id).join(format!(
        "event_{}_discover_videos.json",
        sanitize_timestamp(discovered_at)
    ))
}

fn sanitize_timestamp(value: &str) -> String {
    value.replace(':', "-")
}

async fn write_json_file<T>(path: &Path, value: &T) -> eyre::Result<PathBuf>
where
    T: facet::Facet<'static>,
{
    let content = facet_json::to_string_pretty(value)?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(path, content).await?;
    Ok(path.to_path_buf())
}
