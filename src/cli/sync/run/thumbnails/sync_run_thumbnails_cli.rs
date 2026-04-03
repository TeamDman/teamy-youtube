use arbitrary::Arbitrary;
use chrono::DateTime;
use chrono::Local;
use chrono::TimeDelta;
use chrono::Utc;
use facet::Facet;
use figue as args;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;
use tracing::info;

// yt[sync.thumbnails.command]
/// Download thumbnail assets for videos that already have raw fetch data.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct SyncRunThumbnailsArgs {
    // yt[sync.thumbnails.refresh-video-age]
    /// Refresh thumbnails only for videos newer than this age, for example `2d` or `12h`.
    #[facet(args::named)]
    pub refresh_videos_newer_than: Option<String>,

    // yt[sync.thumbnails.refresh-thumbnail-age]
    /// Refresh thumbnails only when the latest thumbnail observation is older than this age.
    #[facet(args::named)]
    pub refresh_thumbnails_older_than: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct ThumbnailRefreshPolicy {
    refresh_videos_newer_than: TimeDelta,
    refresh_thumbnails_older_than: TimeDelta,
}

#[derive(Debug, Default, Eq, PartialEq)]
struct ThumbnailSyncCounts {
    source_videos: usize,
    discovered: usize,
    existing: usize,
    refresh_eligible: usize,
    unchanged: usize,
    downloaded: usize,
}

#[derive(Debug, Default, Eq, PartialEq)]
struct ThumbnailVideoOutcome {
    counts: ThumbnailSyncCounts,
    bytes_processed: u64,
    last_written_file: Option<String>,
}

struct VideoThumbnailContext<'a> {
    sync_dir: &'a Path,
    client: &'a reqwest::Client,
    video_id: &'a str,
    refresh_policy: Option<&'a ThumbnailRefreshPolicy>,
    published_at: Option<&'a str>,
    thumbnail_index: &'a crate::fs_db::VideoThumbnailIndex,
}

impl SyncRunThumbnailsArgs {
    // yt[sync.thumbnails.latest-fetch]
    /// # Errors
    ///
    /// This function will return an error if the sync dir is unset, raw fetch data cannot be
    /// read, thumbnail URLs cannot be parsed, or thumbnail files cannot be downloaded or written.
    pub async fn invoke(self) -> eyre::Result<()> {
        let refresh_videos_newer_than = self.refresh_videos_newer_than.clone();
        let refresh_thumbnails_older_than = self.refresh_thumbnails_older_than.clone();
        let refresh_policy = parse_refresh_policy(
            refresh_videos_newer_than.as_deref(),
            refresh_thumbnails_older_than.as_deref(),
        )?;
        let sync_dir = crate::paths::try_get_sync_dir()?;
        let video_ids = crate::fs_db::load_video_ids_from_sync_dir(&sync_dir)?;
        let client = reqwest::Client::new();
        let started_at = Instant::now();
        let mut progress = crate::sync_progress::SyncProgress::new(video_ids.len());

        let mut counts = ThumbnailSyncCounts::default();

        info!(
            sync_dir = %sync_dir.display(),
            candidate_video_count = video_ids.len(),
            refresh_videos_newer_than = format_optional_age(refresh_videos_newer_than.as_deref()),
            refresh_thumbnails_older_than = format_optional_age(refresh_thumbnails_older_than.as_deref()),
            "starting sync thumbnails"
        );

        for video_id in &video_ids {
            let video_outcome = process_video_thumbnails(
                &sync_dir,
                &client,
                video_id.as_str(),
                refresh_policy.as_ref(),
            )
            .await?;
            counts.source_videos += video_outcome.counts.source_videos;
            counts.discovered += video_outcome.counts.discovered;
            counts.existing += video_outcome.counts.existing;
            counts.refresh_eligible += video_outcome.counts.refresh_eligible;
            counts.unchanged += video_outcome.counts.unchanged;
            counts.downloaded += video_outcome.counts.downloaded;
            progress.record_item(
                video_outcome.bytes_processed,
                video_outcome.last_written_file,
            );
            progress.emit_log("sync thumbnails progress", started_at.elapsed());
        }

        println!("sync-dir={}", sync_dir.display());
        println!("candidate-video-count={}", video_ids.len());
        println!(
            "thumbnail-fetch-source-video-count={}",
            counts.source_videos
        );
        println!("thumbnail-discovered-count={}", counts.discovered);
        println!("thumbnail-existing-count={}", counts.existing);
        println!(
            "thumbnail-refresh-eligible-count={}",
            counts.refresh_eligible
        );
        println!("thumbnail-unchanged-count={}", counts.unchanged);
        println!("thumbnail-downloaded-count={}", counts.downloaded);
        println!(
            "refresh-videos-newer-than={}",
            format_optional_age(refresh_videos_newer_than.as_deref())
        );
        println!(
            "refresh-thumbnails-older-than={}",
            format_optional_age(refresh_thumbnails_older_than.as_deref())
        );
        Ok(())
    }
}

async fn process_video_thumbnails(
    sync_dir: &Path,
    client: &reqwest::Client,
    video_id: &str,
    refresh_policy: Option<&ThumbnailRefreshPolicy>,
) -> eyre::Result<ThumbnailVideoOutcome> {
    let Some(fetch_event_path) =
        crate::fs_db::latest_successful_video_fetch_event_path(sync_dir, video_id)?
    else {
        return Ok(ThumbnailVideoOutcome::default());
    };

    let raw_response_body = tokio::fs::read_to_string(&fetch_event_path).await?;
    let published_at =
        crate::youtube_api::extract_published_at_from_video_response(&raw_response_body)?;
    let thumbnails =
        crate::youtube_api::extract_thumbnails_from_video_response(&raw_response_body)?;
    let thumbnail_index = crate::fs_db::load_video_thumbnail_index(sync_dir, video_id)?;

    let context = VideoThumbnailContext {
        sync_dir,
        client,
        video_id,
        refresh_policy,
        published_at: published_at.as_deref(),
        thumbnail_index: &thumbnail_index,
    };
    let mut outcome = ThumbnailVideoOutcome {
        counts: ThumbnailSyncCounts {
            source_videos: 1,
            discovered: thumbnails.len(),
            ..ThumbnailSyncCounts::default()
        },
        ..ThumbnailVideoOutcome::default()
    };

    for thumbnail in thumbnails {
        process_single_thumbnail(&context, thumbnail, &mut outcome).await?;
    }

    Ok(outcome)
}

async fn process_single_thumbnail(
    context: &VideoThumbnailContext<'_>,
    thumbnail: crate::youtube_api::YouTubeThumbnail,
    outcome: &mut ThumbnailVideoOutcome,
) -> eyre::Result<()> {
    let thumbnail_size = thumbnail.size_key();
    let existing_asset = context.thumbnail_index.latest_asset_for(&thumbnail_size);
    let latest_observation = context
        .thumbnail_index
        .latest_observation_for(&thumbnail_size);

    // yt[sync.thumbnails.default-no-refetch]
    if existing_asset.is_some()
        && !should_refresh_thumbnail(
            context.refresh_policy,
            context.published_at,
            latest_observation.map(|value| value.observed_at.as_str()),
        )?
    {
        outcome.counts.existing += 1;
        return Ok(());
    }

    if existing_asset.is_some() {
        outcome.counts.refresh_eligible += 1;
    }

    let observed_at = Local::now().to_rfc3339();
    let bytes = context
        .client
        .get(&thumbnail.url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    let downloaded_bytes = u64::try_from(bytes.len())?;

    if let Some(existing_asset) = existing_asset {
        let existing_bytes = tokio::fs::read(&existing_asset.path).await?;
        if existing_bytes == bytes {
            let (unchanged_event_path, event_bytes) = write_unchanged_thumbnail_event(
                context.sync_dir,
                &observed_at,
                context.video_id,
                &thumbnail,
                &thumbnail_size,
                &existing_asset.path,
            )
            .await?;
            outcome.counts.unchanged += 1;
            outcome.bytes_processed += event_bytes;
            outcome.last_written_file = Some(unchanged_event_path.display().to_string());
            return Ok(());
        }
    }

    // yt[sync.thumbnails.event-assets]
    let thumbnail_path = crate::fs_db::video_thumbnail_path_for(
        context.sync_dir,
        context.video_id,
        &observed_at,
        &thumbnail_size,
        &thumbnail.url,
    );
    if let Some(parent) = thumbnail_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    tokio::fs::write(&thumbnail_path, bytes).await?;
    outcome.counts.downloaded += 1;
    outcome.bytes_processed += downloaded_bytes;
    outcome.last_written_file = Some(thumbnail_path.display().to_string());
    Ok(())
}

fn parse_refresh_policy(
    refresh_videos_newer_than: Option<&str>,
    refresh_thumbnails_older_than: Option<&str>,
) -> eyre::Result<Option<ThumbnailRefreshPolicy>> {
    // yt[sync.thumbnails.refresh-requires-both-ages]
    match (refresh_videos_newer_than, refresh_thumbnails_older_than) {
        (None, None) => Ok(None),
        (Some(_), None) | (None, Some(_)) => eyre::bail!(
            "--refresh-videos-newer-than and --refresh-thumbnails-older-than must be provided together"
        ),
        (Some(video_age), Some(thumbnail_age)) => Ok(Some(ThumbnailRefreshPolicy {
            refresh_videos_newer_than: parse_age_argument(video_age, "refresh-videos-newer-than")?,
            refresh_thumbnails_older_than: parse_age_argument(
                thumbnail_age,
                "refresh-thumbnails-older-than",
            )?,
        })),
    }
}

fn parse_age_argument(value: &str, argument_name: &str) -> eyre::Result<TimeDelta> {
    let duration = humantime::parse_duration(value)
        .map_err(|error| eyre::eyre!("invalid value for --{argument_name}: {error}"))?;
    TimeDelta::from_std(duration)
        .map_err(|error| eyre::eyre!("invalid value for --{argument_name}: {error}"))
}

fn should_refresh_thumbnail(
    refresh_policy: Option<&ThumbnailRefreshPolicy>,
    published_at: Option<&str>,
    latest_thumbnail_observed_at: Option<&str>,
) -> eyre::Result<bool> {
    let Some(refresh_policy) = refresh_policy else {
        return Ok(false);
    };
    let Some(published_at) = published_at else {
        return Ok(false);
    };
    let Some(latest_thumbnail_observed_at) = latest_thumbnail_observed_at else {
        return Ok(true);
    };

    let now = Utc::now();
    let video_published_at = DateTime::parse_from_rfc3339(published_at)
        .map_err(|error| eyre::eyre!("failed parsing video published-at timestamp: {error}"))?
        .with_timezone(&Utc);
    let thumbnail_observed_at =
        crate::fs_db::parse_sanitized_event_timestamp(latest_thumbnail_observed_at)?
            .with_timezone(&Utc);

    let video_age = now.signed_duration_since(video_published_at);
    let thumbnail_age = now.signed_duration_since(thumbnail_observed_at);

    Ok(video_age <= refresh_policy.refresh_videos_newer_than
        && thumbnail_age >= refresh_policy.refresh_thumbnails_older_than)
}

fn format_optional_age(value: Option<&str>) -> &str {
    value.unwrap_or("none")
}

// yt[sync.thumbnails.unchanged-event]
async fn write_unchanged_thumbnail_event(
    sync_dir: &Path,
    observed_at: &str,
    video_id: &str,
    thumbnail: &crate::youtube_api::YouTubeThumbnail,
    thumbnail_size: &str,
    compared_asset_path: &Path,
) -> eyre::Result<(PathBuf, u64)> {
    let unchanged_event_path = crate::fs_db::video_thumbnail_unchanged_event_path_for(
        sync_dir,
        video_id,
        observed_at,
        thumbnail_size,
    );
    let event_file = crate::fs_db::ThumbnailUnchangedEventFile {
        observed_at: observed_at.to_owned(),
        video_id: video_id.to_owned(),
        thumbnail_size: thumbnail_size.to_owned(),
        width: thumbnail.width,
        height: thumbnail.height,
        source_url: thumbnail.url.clone(),
        compared_asset_path: compared_asset_path.display().to_string(),
    };
    let content = facet_json::to_string_pretty(&event_file)?;
    tokio::fs::write(&unchanged_event_path, content).await?;
    Ok((
        unchanged_event_path,
        u64::try_from(facet_json::to_string_pretty(&event_file)?.len())?,
    ))
}

#[cfg(test)]
mod tests {
    use super::format_optional_age;
    use super::parse_refresh_policy;
    use super::should_refresh_thumbnail;

    #[test]
    fn formats_missing_optional_age() {
        assert_eq!(format_optional_age(None), "none");
    }

    #[test]
    fn requires_both_refresh_arguments() {
        assert!(parse_refresh_policy(Some("2d"), None).is_err());
        assert!(parse_refresh_policy(None, Some("6h")).is_err());
    }

    #[test]
    fn default_mode_skips_refresh_when_thumbnail_exists() {
        assert!(
            !should_refresh_thumbnail(
                None,
                Some("2026-04-03T00:00:00Z"),
                Some("2026-04-03T10-00-00+00-00"),
            )
            .expect("policy should evaluate")
        );
    }

    #[test]
    fn refreshes_when_video_is_new_and_thumbnail_is_old() {
        let policy = parse_refresh_policy(Some("3650d"), Some("1h"))
            .expect("policy should parse")
            .expect("policy should exist");

        assert!(
            should_refresh_thumbnail(
                Some(&policy),
                Some("2026-04-03T00:00:00Z"),
                Some("2026-04-03T10-00-00+00-00"),
            )
            .expect("policy should evaluate")
        );
    }
}
