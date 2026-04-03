use crate::fs_db::VideoEventFile;
use crate::postgres_sync::PostgresSyncSummary;
use crate::postgres_sync::from_row;
use eyre::WrapErr as _;
use std::path::Path;
use std::path::PathBuf;
use tokio_postgres::NoTls;

pub const DATABASE_URL_ENV_VAR: &str = "DATABASE_URL";
pub const TEAMY_YOUTUBE_DATABASE_URL_ENV_VAR: &str = "TEAMY_YOUTUBE_DATABASE_URL";

const CREATE_VIDEO_EVENTS_TABLE_SQL: &str = r"
CREATE TABLE IF NOT EXISTS youtube_video_events (
    event_id TEXT PRIMARY KEY,
    imported_at TEXT NOT NULL,
    source_kind TEXT NOT NULL,
    source_path TEXT NOT NULL,
    video_id TEXT NOT NULL,
    video_title TEXT,
    channel_name TEXT,
    event_kind TEXT NOT NULL,
    event_at TEXT NOT NULL,
    playlist_id TEXT,
    playlist_name TEXT
)
";

const UPSERT_VIDEO_EVENT_SQL: &str = r"
INSERT INTO youtube_video_events (
    event_id,
    imported_at,
    source_kind,
    source_path,
    video_id,
    video_title,
    channel_name,
    event_kind,
    event_at,
    playlist_id,
    playlist_name
) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
ON CONFLICT (event_id) DO UPDATE SET
    imported_at = EXCLUDED.imported_at,
    source_kind = EXCLUDED.source_kind,
    source_path = EXCLUDED.source_path,
    video_id = EXCLUDED.video_id,
    video_title = EXCLUDED.video_title,
    channel_name = EXCLUDED.channel_name,
    event_kind = EXCLUDED.event_kind,
    event_at = EXCLUDED.event_at,
    playlist_id = EXCLUDED.playlist_id,
    playlist_name = EXCLUDED.playlist_name
";

const SELECT_VIDEO_EVENTS_SQL: &str = r"
SELECT
    imported_at,
    source_kind,
    source_path,
    video_id,
    video_title,
    channel_name,
    event_kind,
    event_at,
    playlist_id,
    playlist_name
FROM youtube_video_events
ORDER BY event_at, video_id
";

/// Resolve the Postgres connection string from an explicit argument or environment.
///
/// # Errors
///
/// Returns an error if no usable database URL is available.
pub fn resolve_database_url(explicit: Option<&str>) -> eyre::Result<String> {
    if let Some(value) = explicit {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_owned());
        }
    }

    if let Ok(value) = std::env::var(TEAMY_YOUTUBE_DATABASE_URL_ENV_VAR) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_owned());
        }
    }

    if let Ok(value) = std::env::var(DATABASE_URL_ENV_VAR) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_owned());
        }
    }

    eyre::bail!(
        "Postgres database URL is not set. Pass --database-url or set TEAMY_YOUTUBE_DATABASE_URL or DATABASE_URL"
    );
}

/// Sync generic event data between Postgres and the filesystem database.
///
/// # Errors
///
/// Returns an error if the filesystem database cannot be read, Postgres cannot be queried,
/// or event files cannot be written.
pub async fn sync_postgres(
    sync_dir: &Path,
    database_url: &str,
) -> eyre::Result<PostgresSyncSummary> {
    let fsdb_events = load_event_files_from_sync_dir(sync_dir)?;
    let fsdb_event_count = fsdb_events.len();

    let (client, connection) = tokio_postgres::connect(database_url, NoTls)
        .await
        .wrap_err("failed to connect to Postgres")?;
    tokio::spawn(async move {
        if let Err(error) = connection.await {
            tracing::error!(%error, "Postgres connection task failed");
        }
    });

    client
        .batch_execute(CREATE_VIDEO_EVENTS_TABLE_SQL)
        .await
        .wrap_err("failed creating youtube_video_events table")?;

    for event_file in &fsdb_events {
        let event_id = event_identity_key(event_file)?;
        client
            .execute(
                UPSERT_VIDEO_EVENT_SQL,
                &[
                    &event_id,
                    &event_file.imported_at,
                    &event_file.source_kind,
                    &event_file.source_path,
                    &event_file.video_id,
                    &event_file.video_title,
                    &event_file.channel_name,
                    &event_file.event_kind,
                    &event_file.event_at,
                    &event_file.playlist_id,
                    &event_file.playlist_name,
                ],
            )
            .await
            .wrap_err_with(|| format!("failed upserting event {event_id} to Postgres"))?;
    }

    let rows = client
        .query(SELECT_VIDEO_EVENTS_SQL, &[])
        .await
        .wrap_err("failed querying youtube_video_events from Postgres")?;
    let postgres_events = rows
        .into_iter()
        .map(|row| {
            from_row::<VideoEventFile>(&row).wrap_err("failed deserializing Postgres event row")
        })
        .collect::<eyre::Result<Vec<_>>>()?;

    let mut summary = PostgresSyncSummary {
        fsdb_event_count,
        postgres_upserted_event_count: fsdb_event_count,
        postgres_event_count: postgres_events.len(),
        fsdb_written_event_file_count: 0,
        fsdb_existing_event_file_count: 0,
    };

    for event_file in &postgres_events {
        let event_path = canonical_event_path_for_video_event(sync_dir, event_file)?;
        write_event_file_if_missing(&event_path, event_file, &mut summary).await?;
    }

    Ok(summary)
}

fn load_event_files_from_sync_dir(sync_dir: &Path) -> eyre::Result<Vec<VideoEventFile>> {
    if !sync_dir.exists() {
        return Ok(Vec::new());
    }

    let mut files_to_visit = vec![sync_dir.to_path_buf()];
    let mut event_files = Vec::new();

    while let Some(directory) = files_to_visit.pop() {
        for entry in std::fs::read_dir(&directory)
            .wrap_err_with(|| format!("failed reading {}", directory.display()))?
        {
            let entry =
                entry.wrap_err_with(|| format!("failed reading {}", directory.display()))?;
            let path = entry.path();
            let file_type = entry
                .file_type()
                .wrap_err_with(|| format!("failed reading file type for {}", path.display()))?;
            if file_type.is_dir() {
                files_to_visit.push(path);
                continue;
            }

            if !is_event_file_path(&path) {
                continue;
            }

            let content = std::fs::read_to_string(&path)
                .wrap_err_with(|| format!("failed reading {}", path.display()))?;
            let event_file: VideoEventFile = facet_json::from_str(&content)
                .wrap_err_with(|| format!("failed parsing {}", path.display()))?;
            event_files.push(event_file);
        }
    }

    Ok(event_files)
}

fn is_event_file_path(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(std::ffi::OsStr::to_str) else {
        return false;
    };

    file_name.starts_with("event_")
        && Path::new(file_name)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
}

fn canonical_event_path_for_video_event(
    sync_dir: &Path,
    event_file: &VideoEventFile,
) -> eyre::Result<PathBuf> {
    let event_suffix = match event_file.event_kind.as_str() {
        "watched" => "watched".to_owned(),
        "added-to-playlist" => crate::fs_db::playlist_event_suffix(
            event_file
                .playlist_id
                .as_deref()
                .ok_or_else(|| eyre::eyre!("playlist event is missing playlist_id"))?,
        ),
        other => eyre::bail!("unsupported event kind for Postgres sync: {other}"),
    };

    Ok(crate::fs_db::event_path_for(
        sync_dir,
        event_file.channel_name.as_deref(),
        event_file.video_title.as_deref(),
        &event_file.video_id,
        &event_file.event_at,
        &event_suffix,
    ))
}

fn event_identity_key(event_file: &VideoEventFile) -> eyre::Result<String> {
    match event_file.event_kind.as_str() {
        "watched" => Ok(format!(
            "watch:{}:{}",
            event_file.video_id, event_file.event_at
        )),
        "added-to-playlist" => Ok(format!(
            "playlist:{}:{}:{}",
            event_file.video_id,
            event_file
                .playlist_id
                .as_deref()
                .ok_or_else(|| eyre::eyre!("playlist event is missing playlist_id"))?,
            event_file.event_at,
        )),
        other => eyre::bail!("unsupported event kind for Postgres sync: {other}"),
    }
}

async fn write_event_file_if_missing(
    path: &Path,
    event_file: &VideoEventFile,
    summary: &mut PostgresSyncSummary,
) -> eyre::Result<()> {
    if tokio::fs::try_exists(path).await? {
        summary.fsdb_existing_event_file_count += 1;
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let content = facet_json::to_string_pretty(event_file)?;
    tokio::fs::write(path, content).await?;
    summary.fsdb_written_event_file_count += 1;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::canonical_event_path_for_video_event;
    use super::event_identity_key;
    use super::is_event_file_path;
    use super::load_event_files_from_sync_dir;
    use crate::fs_db::VideoEventFile;
    use std::path::Path;
    use tempfile::TempDir;

    #[test]
    fn builds_watch_event_identity_key() {
        let event_file = VideoEventFile {
            imported_at: "2026-03-30T21:00:00+00:00".to_owned(),
            source_kind: "takeout-watch-history".to_owned(),
            source_path: "watch-history.json".to_owned(),
            video_id: "abc123".to_owned(),
            video_title: Some("Watched Example".to_owned()),
            channel_name: Some("Example Channel".to_owned()),
            event_kind: "watched".to_owned(),
            event_at: "2026-03-01T12:00:00+00:00".to_owned(),
            playlist_id: None,
            playlist_name: None,
        };

        let event_id = event_identity_key(&event_file).expect("event id should build");

        assert_eq!(event_id, "watch:abc123:2026-03-01T12:00:00+00:00");
    }

    #[test]
    fn builds_playlist_event_path_from_payload() {
        let event_file = VideoEventFile {
            imported_at: "2026-03-30T21:00:00+00:00".to_owned(),
            source_kind: "takeout-playlist-membership".to_owned(),
            source_path: "favorites.csv".to_owned(),
            video_id: "abc123".to_owned(),
            video_title: Some("Example".to_owned()),
            channel_name: Some("Example Channel".to_owned()),
            event_kind: "added-to-playlist".to_owned(),
            event_at: "2026-03-01T12:00:00+00:00".to_owned(),
            playlist_id: Some("favorites".to_owned()),
            playlist_name: Some("Favorites".to_owned()),
        };

        let path = canonical_event_path_for_video_event(Path::new("G:/sync-root"), &event_file)
            .expect("event path should build");

        assert_eq!(
            path.display().to_string().replace('\\', "/"),
            "G:/sync-root/channels/example-channel/videos/abc123-example/event_2026-03-01T12-00-00+00-00_added-to-playlist-favorites.json"
        );
    }

    #[test]
    fn identifies_event_file_paths() {
        assert!(is_event_file_path(Path::new(
            "event_2026-03-01T12-00-00+00-00_watched.json"
        )));
        assert!(!is_event_file_path(Path::new(
            "snapshot_2026-03-01T12-00-00+00-00_video.json"
        )));
    }

    #[test]
    fn loads_only_event_files_from_sync_dir() {
        let temp_dir = TempDir::new().expect("temp dir should be created");
        let event_path = temp_dir
            .path()
            .join("channels")
            .join("example-channel")
            .join("videos")
            .join("abc123-example")
            .join("event_2026-03-01T12-00-00+00-00_watched.json");
        let snapshot_path = temp_dir
            .path()
            .join("channels")
            .join("example-channel")
            .join("videos")
            .join("abc123-example")
            .join("snapshot_2026-03-01T12-00-00+00-00_video.json");
        std::fs::create_dir_all(event_path.parent().expect("event parent should exist"))
            .expect("directories should be created");

        let event_file = VideoEventFile {
            imported_at: "2026-03-30T21:00:00+00:00".to_owned(),
            source_kind: "takeout-watch-history".to_owned(),
            source_path: "watch-history.json".to_owned(),
            video_id: "abc123".to_owned(),
            video_title: Some("Watched Example".to_owned()),
            channel_name: Some("Example Channel".to_owned()),
            event_kind: "watched".to_owned(),
            event_at: "2026-03-01T12:00:00+00:00".to_owned(),
            playlist_id: None,
            playlist_name: None,
        };
        std::fs::write(
            &event_path,
            facet_json::to_string_pretty(&event_file).expect("event json should serialize"),
        )
        .expect("event file should write");
        std::fs::write(&snapshot_path, "{}\n").expect("snapshot file should write");

        let event_files =
            load_event_files_from_sync_dir(temp_dir.path()).expect("event files should load");

        assert_eq!(event_files, vec![event_file]);
    }
}
