use tempfile::TempDir;

#[tokio::test]
async fn writes_video_and_channel_metadata_snapshots() {
    let temp_dir = TempDir::new().expect("temp dir should be created");
    let metadata = teamy_youtube::youtube_api::YouTubeFetchedVideoMetadata {
        source_url: "https://example.invalid/youtube-api".to_owned(),
        video_id: "XfcLWVX-hCA".to_owned(),
        title: "Arc Raiders War Tapes 1".to_owned(),
        description: "Example description".to_owned(),
        channel_id: "UCexample123".to_owned(),
        channel_name: "0biwankenobi".to_owned(),
        published_at: "2025-10-01T12:30:00Z".to_owned(),
        duration_iso8601: "PT4M12S".to_owned(),
        view_count: Some(42),
        like_count: Some(7),
        comment_count: Some(3),
        privacy_status: Some("public".to_owned()),
    };

    let (video_snapshot_path, channel_snapshot_path) =
        teamy_youtube::fs_db::write_fetched_video_metadata(
            temp_dir.path(),
            "2026-03-30T20:00:00+00:00",
            &metadata,
        )
        .await
        .expect("snapshot write should succeed");

    assert_eq!(
        video_snapshot_path
            .strip_prefix(temp_dir.path())
            .expect("relative video path")
            .display()
            .to_string()
            .replace('\\', "/"),
        "channels/0biwankenobi/videos/XfcLWVX-hCA-arc-raiders-war-tapes-1/snapshot_2026-03-30T20-00-00+00-00_video.json"
    );
    assert_eq!(
        channel_snapshot_path
            .strip_prefix(temp_dir.path())
            .expect("relative channel path")
            .display()
            .to_string()
            .replace('\\', "/"),
        "channels/0biwankenobi/snapshot_2026-03-30T20-00-00+00-00_channel.json"
    );

    let video_snapshot_content = tokio::fs::read_to_string(&video_snapshot_path)
        .await
        .expect("video snapshot should be readable");
    let channel_snapshot_content = tokio::fs::read_to_string(&channel_snapshot_path)
        .await
        .expect("channel snapshot should be readable");

    let video_snapshot: teamy_youtube::fs_db::VideoMetadataSnapshotFile =
        facet_json::from_str(&video_snapshot_content).expect("video snapshot should parse");
    let channel_snapshot: teamy_youtube::fs_db::ChannelMetadataSnapshotFile =
        facet_json::from_str(&channel_snapshot_content).expect("channel snapshot should parse");

    assert_eq!(video_snapshot.video_id, "XfcLWVX-hCA");
    assert_eq!(video_snapshot.channel_id, "UCexample123");
    assert_eq!(channel_snapshot.channel_name, "0biwankenobi");
    assert_eq!(channel_snapshot.source_kind, "youtube-data-api-video");
}
