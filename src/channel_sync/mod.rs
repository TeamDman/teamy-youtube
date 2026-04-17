mod channel_discovery;
mod channel_sync_storage;
mod channel_sync_target_file;
mod video_download_event_file;

use crate::channel_sync::channel_discovery::DiscoveredChannel;
use crate::channel_sync::channel_discovery::DiscoveredChannelVideo;
use crate::channel_sync::channel_discovery::discover_channel;
use crate::channel_sync::channel_sync_storage::load_channel_sync_targets;
use crate::channel_sync::channel_sync_storage::read_video_download_request_event;
use crate::channel_sync::channel_sync_storage::write_channel_discovery_event;
use crate::channel_sync::channel_sync_storage::write_channel_sync_target;
use crate::channel_sync::channel_sync_storage::write_video_download_completed_event;
use crate::channel_sync::channel_sync_storage::write_video_download_failed_event;
use crate::channel_sync::channel_sync_storage::write_video_download_request_event;
use chrono::Local;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;
use tokio::process::Command;
use tracing::debug;

pub use channel_sync_target_file::*;
pub use video_download_event_file::*;

const KNOWN_MEDIA_FILE_EXTENSIONS: &[&str] = &[
    "mkv", "mp4", "webm", "m4v", "mov", "avi", "m4a", "mp3", "opus", "flac", "wav", "aac",
];
const EXISTING_MEDIA_QUERY_BATCH_SIZE: usize = 64;
const YT_DLP_OUTPUT_TEMPLATE: &str = "%(title)s [%(id)s].%(ext)s";
const COMMAND_OUTPUT_EXCERPT_LIMIT: usize = 8_192;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChannelSyncPlanMode {
    StatusOnly,
    EnqueueMissingRequests,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ChannelSyncPlanSummary {
    pub target_count: usize,
    pub discovered_video_count: usize,
    pub already_on_disk_count: usize,
    pub pending_request_count: usize,
    pub blocked_failure_count: usize,
    pub new_request_count: usize,
    pub download_planned_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChannelSyncPlan {
    pub summary: ChannelSyncPlanSummary,
    pub work_items: Vec<VideoDownloadWorkItem>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ChannelSyncRunSummary {
    pub plan: ChannelSyncPlanSummary,
    pub downloaded_count: usize,
    pub failed_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddedChannelSyncTarget {
    pub target: ChannelSyncTargetFile,
    pub target_path: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VideoDownloadWorkItem {
    pub request: VideoDownloadRequestEventFile,
}

#[derive(Debug, Default, Eq, PartialEq)]
struct VideoDownloadExecutionOutcome {
    downloaded_count: usize,
    failed_count: usize,
    bytes_processed: u64,
    last_written_file: Option<String>,
}

pub async fn add_channel_sync_target(
    sync_dir: &Path,
    requested_input: &str,
    preferred_download_dir: &Path,
) -> eyre::Result<AddedChannelSyncTarget> {
    let discovered_channel = discover_channel(requested_input, Some(1)).await?;
    let target = ChannelSyncTargetFile {
        added_at: Local::now().to_rfc3339(),
        requested_input: requested_input.trim().to_owned(),
        source_url: discovered_channel.source_url,
        channel_id: discovered_channel.channel_id,
        channel_name: discovered_channel.channel_name,
        uploader_id: discovered_channel.uploader_id,
        uploader_url: discovered_channel.uploader_url,
        channel_url: discovered_channel.channel_url,
        preferred_download_dir: preferred_download_dir.display().to_string(),
    };
    let target_path = write_channel_sync_target(sync_dir, &target).await?;

    Ok(AddedChannelSyncTarget {
        target,
        target_path,
    })
}

pub async fn plan_channel_sync(
    sync_dir: &Path,
    mode: ChannelSyncPlanMode,
) -> eyre::Result<ChannelSyncPlan> {
    let targets = load_channel_sync_targets(sync_dir)?;
    let mut summary = ChannelSyncPlanSummary {
        target_count: targets.len(),
        ..ChannelSyncPlanSummary::default()
    };
    let mut work_items_by_video_id = BTreeMap::<String, VideoDownloadWorkItem>::new();

    for target in &targets {
        let discover_started_at = Instant::now();
        debug!(source_url = %target.source_url, "Discovering tracked channel");
        let discovered_channel = discover_channel(&target.source_url, None).await?;
        debug!(
            source_url = %target.source_url,
            discovered_video_count = discovered_channel.entries.len(),
            elapsed_ms = discover_started_at.elapsed().as_millis(),
            "Discovered tracked channel"
        );
        summary.discovered_video_count += discovered_channel.entries.len();
        if matches!(mode, ChannelSyncPlanMode::EnqueueMissingRequests) {
            let discovery_event = build_channel_discovery_event_file(&discovered_channel);
            let _ = write_channel_discovery_event(sync_dir, &discovery_event).await?;
        }

        let discovered_video_ids = discovered_channel
            .entries
            .iter()
            .map(|entry| entry.video_id.clone())
            .collect::<Vec<_>>();
        let resolve_existing_started_at = Instant::now();
        let existing_media_by_video_id = find_existing_media_files_for_videos(
            &discovered_video_ids,
            Path::new(&target.preferred_download_dir),
        )?;
        let already_on_disk_for_target = discovered_video_ids
            .iter()
            .filter(|video_id| {
                existing_media_by_video_id
                    .get(video_id.as_str())
                    .is_some_and(|paths| !paths.is_empty())
            })
            .count();
        debug!(
            source_url = %target.source_url,
            discovered_video_count = discovered_video_ids.len(),
            already_on_disk_count = already_on_disk_for_target,
            unresolved_video_count = discovered_video_ids
                .len()
                .saturating_sub(already_on_disk_for_target),
            elapsed_ms = resolve_existing_started_at.elapsed().as_millis(),
            "Resolved existing media for tracked channel"
        );

        for entry in &discovered_channel.entries {
            if existing_media_by_video_id
                .get(entry.video_id.as_str())
                .is_some_and(|paths| !paths.is_empty())
            {
                summary.already_on_disk_count += 1;
                continue;
            }

            assess_discovered_video(
                sync_dir,
                target,
                entry,
                mode,
                &mut summary,
                &mut work_items_by_video_id,
            )
            .await?;
        }
    }

    let work_items = work_items_by_video_id.into_values().collect::<Vec<_>>();
    summary.download_planned_count = work_items.len();

    Ok(ChannelSyncPlan {
        summary,
        work_items,
    })
}

pub async fn execute_channel_sync_plan(
    sync_dir: &Path,
    plan: &ChannelSyncPlan,
) -> eyre::Result<ChannelSyncRunSummary> {
    let mut summary = ChannelSyncRunSummary {
        plan: plan.summary.clone(),
        ..ChannelSyncRunSummary::default()
    };
    let started_at = Instant::now();
    let mut progress = crate::sync_progress::SyncProgress::new(plan.work_items.len());

    for work_item in &plan.work_items {
        let outcome = execute_video_download_work_item(sync_dir, work_item).await?;
        summary.downloaded_count += outcome.downloaded_count;
        summary.failed_count += outcome.failed_count;
        progress.record_item(outcome.bytes_processed, outcome.last_written_file);
        progress.emit_log("sync channel progress", started_at.elapsed());
    }

    Ok(summary)
}

async fn assess_discovered_video(
    sync_dir: &Path,
    target: &ChannelSyncTargetFile,
    entry: &DiscoveredChannelVideo,
    mode: ChannelSyncPlanMode,
    summary: &mut ChannelSyncPlanSummary,
    work_items_by_video_id: &mut BTreeMap<String, VideoDownloadWorkItem>,
) -> eyre::Result<()> {
    let download_index =
        crate::fs_db::load_video_download_event_index(sync_dir, entry.video_id.as_str())?;

    if download_index.has_pending_request()
        && let Some(request_path) = download_index.latest_request_event_path.as_deref()
    {
        let existing_request = read_video_download_request_event(request_path)?;
        if request_matches_discovered_video(&existing_request, target, entry) {
            summary.pending_request_count += 1;
            work_items_by_video_id
                .entry(existing_request.video_id.clone())
                .or_insert(VideoDownloadWorkItem {
                    request: existing_request,
                });
            return Ok(());
        }
    }

    if download_index.has_blocking_failure() {
        summary.blocked_failure_count += 1;
        return Ok(());
    }

    let request = build_video_download_request(target, entry);
    if matches!(mode, ChannelSyncPlanMode::EnqueueMissingRequests) {
        let _ = write_video_download_request_event(sync_dir, &request).await?;
    }
    summary.new_request_count += 1;
    work_items_by_video_id.insert(request.video_id.clone(), VideoDownloadWorkItem { request });

    Ok(())
}

async fn execute_video_download_work_item(
    sync_dir: &Path,
    work_item: &VideoDownloadWorkItem,
) -> eyre::Result<VideoDownloadExecutionOutcome> {
    let preferred_download_dir = PathBuf::from(&work_item.request.preferred_download_dir);
    std::fs::create_dir_all(&preferred_download_dir)?;

    if let Some(media_path) = select_preferred_media_path(find_local_media_files_in_directory(
        &preferred_download_dir,
        &work_item.request.video_id,
    )?) {
        return write_completed_download_outcome(sync_dir, &work_item.request, &media_path).await;
    }

    let stamp = sanitize_component(&Local::now().to_rfc3339());
    let print_path = crate::paths::CACHE_DIR.0.join("yt-dlp").join(format!(
        "download-{}-{stamp}.txt",
        work_item.request.video_id
    ));
    if let Some(parent) = print_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let output = Command::new("yt-dlp")
        .current_dir(&preferred_download_dir)
        .arg("--windows-filenames")
        .arg("--write-info-json")
        .arg("--write-subs")
        .arg("--write-auto-subs")
        .arg("--embed-metadata")
        .arg("--no-overwrites")
        .arg("--output")
        .arg(YT_DLP_OUTPUT_TEMPLATE)
        .arg("--print-to-file")
        .arg("after_move:%(filepath)s")
        .arg(&print_path)
        .arg(&work_item.request.video_url)
        .output()
        .await?;

    let outcome = if output.status.success() {
        let media_path = read_printed_media_path(&print_path).or_else(|| {
            select_preferred_media_path(
                find_local_media_files_in_directory(
                    &preferred_download_dir,
                    &work_item.request.video_id,
                )
                .ok()?,
            )
        });

        if let Some(media_path) = media_path {
            write_completed_download_outcome(sync_dir, &work_item.request, &media_path).await?
        } else {
            write_failed_download_outcome(
                sync_dir,
                &work_item.request,
                output.status.code(),
                &output.stdout,
                &output.stderr,
            )
            .await?
        }
    } else {
        write_failed_download_outcome(
            sync_dir,
            &work_item.request,
            output.status.code(),
            &output.stdout,
            &output.stderr,
        )
        .await?
    };

    let _ = tokio::fs::remove_file(&print_path).await;
    Ok(outcome)
}

async fn write_completed_download_outcome(
    sync_dir: &Path,
    request: &VideoDownloadRequestEventFile,
    media_path: &Path,
) -> eyre::Result<VideoDownloadExecutionOutcome> {
    let media_bytes = std::fs::metadata(media_path)?.len();
    let event_file = VideoDownloadCompletedEventFile {
        downloaded_at: Local::now().to_rfc3339(),
        source_kind: request.source_kind.clone(),
        source_url: request.source_url.clone(),
        channel_id: request.channel_id.clone(),
        channel_name: request.channel_name.clone(),
        video_id: request.video_id.clone(),
        video_url: request.video_url.clone(),
        video_title: request.video_title.clone(),
        preferred_download_dir: request.preferred_download_dir.clone(),
        media_path: media_path.display().to_string(),
        media_bytes,
    };
    let _event_path = write_video_download_completed_event(sync_dir, &event_file).await?;

    Ok(VideoDownloadExecutionOutcome {
        downloaded_count: 1,
        failed_count: 0,
        bytes_processed: media_bytes,
        last_written_file: Some(media_path.display().to_string()),
    })
}

async fn write_failed_download_outcome(
    sync_dir: &Path,
    request: &VideoDownloadRequestEventFile,
    exit_code: Option<i32>,
    stdout: &[u8],
    stderr: &[u8],
) -> eyre::Result<VideoDownloadExecutionOutcome> {
    let event_file = VideoDownloadFailedEventFile {
        failed_at: Local::now().to_rfc3339(),
        source_kind: request.source_kind.clone(),
        source_url: request.source_url.clone(),
        channel_id: request.channel_id.clone(),
        channel_name: request.channel_name.clone(),
        video_id: request.video_id.clone(),
        video_url: request.video_url.clone(),
        video_title: request.video_title.clone(),
        preferred_download_dir: request.preferred_download_dir.clone(),
        exit_code,
        stdout_excerpt: command_output_excerpt(stdout),
        stderr_excerpt: command_output_excerpt(stderr),
    };
    let event_path = write_video_download_failed_event(sync_dir, &event_file).await?;

    Ok(VideoDownloadExecutionOutcome {
        downloaded_count: 0,
        failed_count: 1,
        bytes_processed: 0,
        last_written_file: Some(event_path.display().to_string()),
    })
}

fn build_channel_discovery_event_file(channel: &DiscoveredChannel) -> ChannelDiscoveryEventFile {
    ChannelDiscoveryEventFile {
        discovered_at: Local::now().to_rfc3339(),
        source_url: channel.source_url.clone(),
        channel_id: channel.channel_id.clone(),
        channel_name: channel.channel_name.clone(),
        discovered_video_count: channel.entries.len(),
        videos: channel
            .entries
            .iter()
            .map(|entry| ChannelDiscoveryEventVideo {
                video_id: entry.video_id.as_str().to_owned(),
                video_url: entry.video_url.clone(),
                video_title: entry.video_title.clone(),
            })
            .collect(),
    }
}

fn build_video_download_request(
    target: &ChannelSyncTargetFile,
    entry: &DiscoveredChannelVideo,
) -> VideoDownloadRequestEventFile {
    VideoDownloadRequestEventFile {
        requested_at: Local::now().to_rfc3339(),
        source_kind: "channel-sync".to_owned(),
        source_url: target.source_url.clone(),
        channel_id: target.channel_id.clone(),
        channel_name: target.channel_name.clone(),
        video_id: entry.video_id.as_str().to_owned(),
        video_url: entry.video_url.clone(),
        video_title: entry.video_title.clone(),
        preferred_download_dir: target.preferred_download_dir.clone(),
    }
}

fn request_matches_discovered_video(
    request: &VideoDownloadRequestEventFile,
    target: &ChannelSyncTargetFile,
    entry: &DiscoveredChannelVideo,
) -> bool {
    request.channel_id == target.channel_id
        && request.video_id == entry.video_id.as_str()
        && request.video_url == entry.video_url
        && request.preferred_download_dir == target.preferred_download_dir
}

fn find_existing_media_files_for_videos(
    video_ids: &[crate::takeout::YouTubeVideoId],
    preferred_download_dir: &Path,
) -> eyre::Result<BTreeMap<String, Vec<PathBuf>>> {
    let unique_video_ids = video_ids
        .iter()
        .map(|video_id| video_id.as_str().to_owned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let local_scan_started_at = Instant::now();
    let mut existing_paths_by_video_id = find_local_media_files_in_directory_for_video_ids(
        preferred_download_dir,
        &unique_video_ids,
    )?;
    debug!(
        directory = %preferred_download_dir.display(),
        video_id_count = unique_video_ids.len(),
        locally_resolved_video_count = existing_paths_by_video_id.len(),
        elapsed_ms = local_scan_started_at.elapsed().as_millis(),
        "Scanned preferred download directory for existing media"
    );

    let unresolved_video_ids = unique_video_ids
        .iter()
        .filter(|video_id| !existing_paths_by_video_id.contains_key(video_id.as_str()))
        .cloned()
        .collect::<Vec<_>>();

    if !unresolved_video_ids.is_empty() {
        debug!(
            directory = %preferred_download_dir.display(),
            unresolved_video_count = unresolved_video_ids.len(),
            batch_size = EXISTING_MEDIA_QUERY_BATCH_SIZE,
            "Falling back to teamy-mft for unresolved existing media lookup"
        );
    }

    for (chunk_index, chunk) in unresolved_video_ids
        .chunks(EXISTING_MEDIA_QUERY_BATCH_SIZE)
        .enumerate()
    {
        let query_started_at = Instant::now();
        let matching_paths = teamy_mft::cli::command::query::QueryArgs {
            query: build_existing_media_queries(chunk),
            ..Default::default()
        }
        .invoke()?;
        debug!(
            chunk_index,
            chunk_size = chunk.len(),
            matched_path_count = matching_paths.len(),
            elapsed_ms = query_started_at.elapsed().as_millis(),
            "Queried teamy-mft for existing media"
        );

        for path in matching_paths {
            for video_id in chunk {
                if path_matches_video_id(&path, video_id) {
                    existing_paths_by_video_id
                        .entry(video_id.clone())
                        .or_default()
                        .push(path.clone());
                }
            }
        }
    }

    for paths in existing_paths_by_video_id.values_mut() {
        paths.sort();
        paths.dedup();
    }

    Ok(existing_paths_by_video_id)
}

fn build_existing_media_queries(video_ids: &[String]) -> Vec<String> {
    video_ids.to_vec()
}

fn find_local_media_files_in_directory_for_video_ids(
    directory: &Path,
    video_ids: &[String],
) -> eyre::Result<BTreeMap<String, Vec<PathBuf>>> {
    if !directory.is_dir() || video_ids.is_empty() {
        return Ok(BTreeMap::new());
    }

    let mut paths_by_video_id = BTreeMap::<String, Vec<PathBuf>>::new();
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }

        let path = entry.path();
        if !has_known_media_file_extension(&path) {
            continue;
        }

        for video_id in video_ids {
            if path_matches_video_id(&path, video_id) {
                paths_by_video_id
                    .entry(video_id.clone())
                    .or_default()
                    .push(path.clone());
            }
        }
    }

    for paths in paths_by_video_id.values_mut() {
        paths.sort();
        paths.dedup();
    }

    Ok(paths_by_video_id)
}

fn path_matches_video_id(path: &Path, video_id: &str) -> bool {
    let Some(file_stem) = path.file_stem().and_then(std::ffi::OsStr::to_str) else {
        return false;
    };

    has_known_media_file_extension(path) && file_stem_matches_video_id(file_stem, video_id)
}

fn file_stem_matches_video_id(file_stem: &str, video_id: &str) -> bool {
    file_stem == video_id || file_stem.ends_with(&format!("[{video_id}]"))
}

fn has_known_media_file_extension(path: &Path) -> bool {
    path.extension()
        .and_then(std::ffi::OsStr::to_str)
        .map(str::to_ascii_lowercase)
        .is_some_and(|extension| KNOWN_MEDIA_FILE_EXTENSIONS.contains(&extension.as_str()))
}

fn find_local_media_files_in_directory(
    directory: &Path,
    video_id: &str,
) -> eyre::Result<Vec<PathBuf>> {
    if !directory.is_dir() {
        return Ok(Vec::new());
    }

    let mut paths = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }

        let path = entry.path();
        if path_matches_video_id(&path, video_id) {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn select_preferred_media_path(paths: Vec<PathBuf>) -> Option<PathBuf> {
    paths.into_iter().max_by_key(|path| {
        std::fs::metadata(path)
            .map(|metadata| metadata.len())
            .unwrap_or(0)
    })
}

fn read_printed_media_path(path: &Path) -> Option<PathBuf> {
    let content = std::fs::read_to_string(path).ok()?;
    let line = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .next_back()?;
    let media_path = PathBuf::from(line);
    (media_path.is_file() && has_known_media_file_extension(&media_path)).then_some(media_path)
}

fn command_output_excerpt(bytes: &[u8]) -> Option<String> {
    let content = String::from_utf8_lossy(bytes).trim().to_owned();
    if content.is_empty() {
        return None;
    }

    let mut excerpt = String::new();
    for character in content.chars().take(COMMAND_OUTPUT_EXCERPT_LIMIT) {
        excerpt.push(character);
    }
    Some(excerpt)
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

    sanitized.trim_matches('-').to_owned()
}

#[cfg(test)]
mod tests {
    use super::build_existing_media_queries;
    use super::find_local_media_files_in_directory_for_video_ids;
    use super::path_matches_video_id;
    use std::path::Path;

    #[test]
    fn existing_media_queries_use_raw_video_ids_for_fast_mft_lookup() {
        let queries = build_existing_media_queries(&["abc123def45".to_owned()]);

        assert_eq!(queries, vec!["abc123def45".to_owned()]);
    }

    #[test]
    fn path_matching_requires_canonical_video_id_suffix() {
        assert!(path_matches_video_id(
            Path::new("Example [abc123def45].mkv"),
            "abc123def45"
        ));
        assert!(path_matches_video_id(
            Path::new("abc123def45.webm"),
            "abc123def45"
        ));
        assert!(!path_matches_video_id(
            Path::new("Example abc123def45 trailer.mkv"),
            "abc123def45"
        ));
        assert!(!path_matches_video_id(
            Path::new("abc123def45-extra.mkv"),
            "abc123def45"
        ));
        assert!(!path_matches_video_id(
            Path::new("abc123def45.txt"),
            "abc123def45"
        ));
    }

    #[test]
    fn preferred_download_directory_scan_detects_existing_media_by_video_id() -> eyre::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        std::fs::write(temp_dir.path().join("Example [abc123def45].mkv"), b"ok")?;
        std::fs::write(temp_dir.path().join("Other [zzz98765432].webm"), b"ok")?;
        std::fs::write(
            temp_dir.path().join("Ignore [abc123def45].info.json"),
            b"ok",
        )?;

        let existing = find_local_media_files_in_directory_for_video_ids(
            temp_dir.path(),
            &[
                "abc123def45".to_owned(),
                "zzz98765432".to_owned(),
                "missing0000".to_owned(),
            ],
        )?;

        assert_eq!(existing.get("abc123def45").map(Vec::len), Some(1));
        assert_eq!(existing.get("zzz98765432").map(Vec::len), Some(1));
        assert!(!existing.contains_key("missing0000"));
        Ok(())
    }
}
