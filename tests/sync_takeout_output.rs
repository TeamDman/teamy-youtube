use std::path::Path;
use tempfile::TempDir;

#[tokio::test]
async fn sync_takeout_writes_video_id_prefixed_paths_and_playlist_events() {
    let temp_dir = TempDir::new().expect("temp dir should be created");
    let takeout_root = temp_dir
        .path()
        .join("Takeout")
        .join("YouTube and YouTube Music");
    let history_dir = takeout_root.join("history");
    let playlists_dir = takeout_root.join("playlists");
    std::fs::create_dir_all(&history_dir).expect("history dir should be created");
    std::fs::create_dir_all(&playlists_dir).expect("playlists dir should be created");

    let watch_history_path = history_dir.join("watch-history.json");
    let playlist_csv_path = playlists_dir.join("Favorites-videos.csv");
    std::fs::write(
        &watch_history_path,
        r#"[
  {
    "header": "YouTube",
    "title": "Watched Arc Raiders War Tapes 1",
    "titleUrl": "https://www.youtube.com/watch?v=XfcLWVX-hCA",
    "subtitles": [
      {
        "name": "0biwankenobi",
        "url": "https://www.youtube.com/channel/example"
      }
    ],
    "time": "2025-11-20T23:03:24.580+00:00"
  }
]"#,
    )
    .expect("watch history should be written");
    std::fs::write(
        &playlist_csv_path,
        "Video ID,Playlist Video Creation Timestamp\nXfcLWVX-hCA,2026-02-20T18:22:41+00:00\n",
    )
    .expect("playlist csv should be written");

    let playlist_entries = teamy_youtube::takeout::read_playlist_video_entries(
        &playlist_csv_path,
        "Favorites".to_owned(),
    )
    .await
    .expect("playlist entries should parse");
    let watch_history_report =
        teamy_youtube::takeout::read_watch_history_entries(&watch_history_path)
            .await
            .expect("watch history should parse");

    let sync_dir = temp_dir.path().join("sync-db");
    let summary = teamy_youtube::fs_db::write_takeout_sync(
        &sync_dir,
        "2026-03-29T12:00:00+00:00",
        &watch_history_path,
        &playlist_entries,
        &watch_history_report.entries,
        false,
    )
    .await
    .expect("sync write should succeed");

    assert_eq!(summary.watch_event_count, 1);
    assert_eq!(summary.playlist_event_count, 1);
    assert_eq!(summary.written_event_file_count, 2);

    let video_dir = sync_dir.join("videos").join("XfcLWVX-hCA");
    assert!(
        video_dir.exists(),
        "expected video directory at {}",
        video_dir.display()
    );

    let watched_event_path = video_dir.join("event_2025-11-20T23-03-24.580+00-00_watched.json");
    let playlist_event_path =
        video_dir.join("event_2026-02-20T18-22-41+00-00_added-to-playlist-favorites.json");

    assert!(watched_event_path.exists(), "missing watched event file");
    assert!(playlist_event_path.exists(), "missing playlist event file");

    let watched_event = read_event_file(&watched_event_path).await;
    let playlist_event = read_event_file(&playlist_event_path).await;

    assert_eq!(watched_event.event_kind, "watched");
    assert_eq!(watched_event.video_id, "XfcLWVX-hCA");
    assert_eq!(playlist_event.event_kind, "added-to-playlist");
    assert_eq!(playlist_event.playlist_id.as_deref(), Some("favorites"));
    assert_eq!(playlist_event.playlist_name.as_deref(), Some("Favorites"));
}

#[tokio::test]
async fn sync_takeout_uses_video_id_only_when_title_is_unknown() {
    let temp_dir = TempDir::new().expect("temp dir should be created");
    let sync_dir = temp_dir.path().join("sync-db");

    let playlist_entries = vec![
        teamy_youtube::takeout::PlaylistVideoEntry::parse_csv_line(
            2,
            "abc123xyz89,2026-02-20T18:22:41+00:00",
            "watch-later".to_owned(),
            "Watch Later".to_owned(),
            "playlist.csv".to_owned(),
        )
        .expect("playlist entry should parse"),
    ];

    let summary = teamy_youtube::fs_db::write_takeout_sync(
        &sync_dir,
        "2026-03-29T12:00:00+00:00",
        Path::new("watch-history.json"),
        &playlist_entries,
        &[],
        false,
    )
    .await
    .expect("sync write should succeed");

    assert_eq!(summary.watch_event_count, 0);
    assert_eq!(summary.playlist_event_count, 1);

    let video_dir = sync_dir.join("videos").join("abc123xyz89");
    let playlist_event_path =
        video_dir.join("event_2026-02-20T18-22-41+00-00_added-to-playlist-watch-later.json");

    assert!(
        playlist_event_path.exists(),
        "missing playlist-only event file"
    );
}

async fn read_event_file(path: &Path) -> teamy_youtube::fs_db::VideoEventFile {
    let content = tokio::fs::read_to_string(path)
        .await
        .expect("event file should be readable");
    facet_json::from_str(&content).expect("event file should parse")
}
