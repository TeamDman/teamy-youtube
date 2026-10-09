use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use teamy_youtube::takeout_zip::*;
use tempfile::TempDir;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

const HISTORY: &str = "Takeout/YouTube and YouTube Music/history/watch-history.json";
const RECORDS: &str = r#"[
 {"title":"Watched Synthetic Video","titleUrl":"https://www.youtube.com/watch?v=abc123def45","subtitles":[{"name":"Synthetic Channel","url":"https://www.youtube.com/channel/UCabcdefghijklmnopqrstuv"}],"time":"2026-01-02T03:04:05Z","products":["YouTube"]},
 {"title":"Watched Without Channel","titleUrl":"https://www.youtube.com/watch?v=xyz987uvw65","time":"2026-01-03T03:04:05+00:00"},
 {"title":"Watched Without URL","time":"2026-01-04T03:04:05Z"}
]"#;

fn zip_fixture(path: &Path, entries: &[(&str, &str)]) {
    let mut writer = ZipWriter::new(File::create(path).unwrap());
    for (name, text) in entries {
        writer
            .start_file(
                *name,
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
            )
            .unwrap();
        writer.write_all(text.as_bytes()).unwrap();
    }
    writer.finish().unwrap();
}

fn select(path: &Path) -> SelectedArchive {
    select_archive(
        &ArchiveSelectionRequest {
            zip: Some(path.to_path_buf()),
            input_dirs: Vec::new(),
            mft_max_age_seconds: 7200,
        },
        &ArchiveReadLimits::default(),
    )
    .unwrap()
}

#[test]
fn explicit_history_only_archive_keeps_optional_fields_and_provenance() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("renamed.zip");
    zip_fixture(
        &path,
        &[
            (HISTORY, RECORDS),
            ("Takeout/other/unrelated.bin", "not JSON"),
        ],
    );
    let selected = select(&path);
    assert_eq!(selected.discovery_source, "explicit-path");
    let before = std::fs::read(&path).unwrap();
    let result = read_archive(ArchiveReadRequest {
        selected,
        limits: ArchiveReadLimits::default(),
    })
    .unwrap();
    assert_eq!(result.records.len(), 2);
    assert_eq!(result.skipped_record_count, 1);
    let record = &result.records[0];
    assert_eq!(record.video_id, "abc123def45");
    assert_eq!(record.watched_at, "2026-01-02T03:04:05+00:00");
    assert_eq!(
        record.channel_id.as_deref(),
        Some("UCabcdefghijklmnopqrstuv")
    );
    assert_eq!(record.channel_name.as_deref(), Some("Synthetic Channel"));
    assert_eq!(record.entry_name, HISTORY);
    assert_eq!(record.record_index, 0);
    assert_eq!(result.records[1].channel_name, None);
    assert_eq!(result.records[1].channel_url, None);
    assert_eq!(result.records[1].channel_id, None);
    assert_eq!(before, std::fs::read(path).unwrap());
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);
}

#[test]
fn optional_subtitle_url_and_handle_reference_do_not_invent_channel_ids() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("test.zip");
    let json = RECORDS.replace(
        ",\"url\":\"https://www.youtube.com/channel/UCabcdefghijklmnopqrstuv\"",
        "",
    );
    zip_fixture(&path, &[(HISTORY, &json)]);
    let result = read_archive(ArchiveReadRequest {
        selected: select(&path),
        limits: ArchiveReadLimits::default(),
    })
    .unwrap();
    assert_eq!(result.records[0].channel_id, None);
    let json = RECORDS.replace("/channel/UCabcdefghijklmnopqrstuv", "/@synthetic");
    zip_fixture(&path, &[(HISTORY, &json)]);
    let result = read_archive(ArchiveReadRequest {
        selected: select(&path),
        limits: ArchiveReadLimits::default(),
    })
    .unwrap();
    assert!(
        result.records[0]
            .channel_url
            .as_ref()
            .unwrap()
            .ends_with("/@synthetic")
    );
    assert_eq!(result.records[0].channel_id, None);
}

#[test]
fn automatic_selection_is_order_independent_and_excludes_unrelated_archives() {
    let temp = TempDir::new().unwrap();
    let older = temp.path().join("takeout-20260101T000000Z-001.zip");
    let newer = temp.path().join("takeout-20260201T000000Z-001.zip");
    let unrelated = temp.path().join("takeout-20260301T000000Z-001.zip");
    zip_fixture(&older, &[(HISTORY, "[]")]);
    zip_fixture(&newer, &[(HISTORY, "[]")]);
    zip_fixture(&unrelated, &[("Takeout/Drive/other.json", "{}")]);
    let paths = vec![newer.clone(), unrelated.clone(), older.clone()];
    let selected = select_archive_from_candidates(
        paths.clone(),
        "synthetic-candidates",
        Vec::new(),
        &ArchiveReadLimits::default(),
    )
    .unwrap();
    let reversed = select_archive_from_candidates(
        paths.into_iter().rev().collect(),
        "synthetic-candidates",
        Vec::new(),
        &ArchiveReadLimits::default(),
    )
    .unwrap();
    assert_eq!(selected, reversed);
    assert_eq!(
        selected.filename_timestamp.as_deref(),
        Some("2026-02-01T00:00:00Z")
    );
    assert_eq!(selected.candidates.len(), 3);
    assert!(
        selected
            .candidates
            .iter()
            .any(|candidate| candidate.status == "ignored-no-youtube-watch-history-json")
    );
    let from_dirs = select_archive(
        &ArchiveSelectionRequest {
            zip: None,
            input_dirs: vec![temp.path().to_path_buf()],
            mft_max_age_seconds: 7200,
        },
        &ArchiveReadLimits::default(),
    )
    .unwrap();
    assert_eq!(from_dirs.snapshot, selected.snapshot);
}

#[test]
fn automatic_selection_rejects_ties_and_nonstandard_names() {
    let temp = TempDir::new().unwrap();
    let first = temp.path().join("takeout-20260101T000000Z-001.zip");
    let second = temp.path().join("takeout-20260101T000000Z-002.zip");
    zip_fixture(&first, &[(HISTORY, "[]")]);
    zip_fixture(&second, &[(HISTORY, "[]")]);
    let error = select_archive_from_candidates(
        vec![first, second],
        "test",
        Vec::new(),
        &ArchiveReadLimits::default(),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("Ambiguous newest"));
    let renamed = temp.path().join("renamed.zip");
    zip_fixture(&renamed, &[(HISTORY, "[]")]);
    assert!(
        select_archive_from_candidates(
            vec![renamed],
            "test",
            Vec::new(),
            &ArchiveReadLimits::default()
        )
        .unwrap_err()
        .to_string()
        .contains("No eligible")
    );
    let _ = select_archive_from_candidates(
        Vec::new(),
        "test",
        Vec::new(),
        &ArchiveReadLimits::default(),
    )
    .unwrap_err();
}

#[test]
fn inspection_rejects_unsafe_oversized_and_ambiguous_entries() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("test.zip");
    for name in [
        "../escape.json",
        "/absolute.json",
        "C:/drive.json",
        "folder/../escape.json",
        "folder\\..\\escape.json",
    ] {
        zip_fixture(&path, &[(HISTORY, "[]"), (name, "{}")]);
        assert!(
            inspect_archive(select(&path), &ArchiveReadLimits::default())
                .unwrap_err()
                .to_string()
                .contains("Unsafe")
        );
    }
    zip_fixture(&path, &[(HISTORY, RECORDS)]);
    let limits = ArchiveReadLimits {
        max_entry_bytes: 10,
        ..ArchiveReadLimits::default()
    };
    assert!(
        inspect_archive(select(&path), &limits)
            .unwrap_err()
            .to_string()
            .contains("byte limit")
    );
    zip_fixture(
        &path,
        &[
            (HISTORY, "[]"),
            ("Other/YouTube/history/watch-history.json", "[]"),
        ],
    );
    assert!(
        inspect_archive(select(&path), &ArchiveReadLimits::default())
            .unwrap_err()
            .to_string()
            .contains("Ambiguous")
    );
    let limits = ArchiveReadLimits {
        max_entries: 1,
        ..ArchiveReadLimits::default()
    };
    assert!(
        inspect_archive(select(&path), &limits)
            .unwrap_err()
            .to_string()
            .contains("entry-count")
    );
}

#[test]
fn malformed_history_is_not_decoded_by_inspect_and_errors_omit_record_contents() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("test.zip");
    zip_fixture(&path, &[(HISTORY, "private-marker malformed JSON")]);
    let selected = select(&path);
    inspect_archive(selected.clone(), &ArchiveReadLimits::default()).unwrap();
    let error = read_archive(ArchiveReadRequest {
        selected,
        limits: ArchiveReadLimits::default(),
    })
    .unwrap_err()
    .to_string();
    assert!(error.contains("Malformed watch-history JSON"));
    assert!(!error.contains("private-marker"));
    zip_fixture(
        &path,
        &[(
            HISTORY,
            &RECORDS.replace("2026-01-02T03:04:05Z", "invalid-time"),
        )],
    );
    assert!(
        read_archive(ArchiveReadRequest {
            selected: select(&path),
            limits: ArchiveReadLimits::default()
        })
        .unwrap_err()
        .to_string()
        .contains("record 0")
    );
    std::fs::write(&path, b"not a ZIP").unwrap();
    let _ = inspect_archive(select(&path), &ArchiveReadLimits::default()).unwrap_err();
}

#[test]
fn changed_snapshot_and_missing_json_are_actionable() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("test.zip");
    zip_fixture(&path, &[(HISTORY, "[]")]);
    let selected = select(&path);
    zip_fixture(&path, &[(HISTORY, RECORDS)]);
    assert!(
        inspect_archive(selected, &ArchiveReadLimits::default())
            .unwrap_err()
            .to_string()
            .contains("changed since selection")
    );
    zip_fixture(
        &path,
        &[(
            "Takeout/YouTube/history/watch-history.html",
            "<html></html>",
        )],
    );
    let selected = select(&path);
    inspect_archive(selected.clone(), &ArchiveReadLimits::default()).unwrap();
    assert!(
        read_archive(ArchiveReadRequest {
            selected,
            limits: ArchiveReadLimits::default()
        })
        .unwrap_err()
        .to_string()
        .contains("HTML history is not supported")
    );
}

fn command(temp: &TempDir, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_teamy-youtube"))
        .args(args)
        .env(
            "TEAMY_YOUTUBE_HOME",
            temp.path().join("home-must-not-be-created"),
        )
        .env(
            "TEAMY_YOUTUBE_CACHE_DIR",
            temp.path().join("cache-must-not-be-created"),
        )
        .env(
            "TEAMY_YOUTUBE_SYNC_DIR",
            temp.path().join("sync-must-not-be-created"),
        )
        .env_remove("TEAMY_YOUTUBE_TAKEOUT_DIR")
        .env_remove("YOUTUBE_API_KEY")
        .output()
        .unwrap()
}

#[test]
fn cli_structured_json_csv_and_text_are_read_only() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("takeout-20260101T000000Z-001.zip");
    zip_fixture(&path, &[(HISTORY, RECORDS)]);
    let path_text = path.to_str().unwrap();
    let before = std::fs::read(&path).unwrap();
    let result = command(&temp, &["takeout", "read", path_text]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let json = String::from_utf8(result.stdout).unwrap();
    let decoded: ArchiveReadResult = facet_json::from_str(&json).unwrap();
    assert_eq!(decoded.records.len(), 2);
    assert_eq!(
        decoded.inspection.selected.discovery_source,
        "explicit-path"
    );
    let again = command(
        &temp,
        &["takeout", "read", path_text, "--output-format", "json"],
    );
    assert_eq!(json.as_bytes(), again.stdout);
    let csv = command(
        &temp,
        &["takeout", "read", path_text, "--output-format", "csv"],
    );
    assert!(
        csv.status.success(),
        "{}",
        String::from_utf8_lossy(&csv.stderr)
    );
    let csv = String::from_utf8(csv.stdout).unwrap();
    assert!(csv.contains("video-id") && csv.contains("abc123def45") && csv.contains("entry-name"));
    let inspect = command(
        &temp,
        &["takeout", "inspect", path_text, "--output-format", "csv"],
    );
    assert!(
        inspect.status.success(),
        "{}",
        String::from_utf8_lossy(&inspect.stderr)
    );
    assert!(
        String::from_utf8(inspect.stdout)
            .unwrap()
            .contains("compressed-bytes")
    );
    let text = command(
        &temp,
        &["takeout", "inspect", path_text, "--output-format", "text"],
    );
    assert!(text.status.success());
    let auto = command(
        &temp,
        &[
            "takeout",
            "inspect",
            "--input-dir",
            temp.path().to_str().unwrap(),
        ],
    );
    assert!(
        auto.status.success(),
        "{}",
        String::from_utf8_lossy(&auto.stderr)
    );
    let conflict = command(
        &temp,
        &[
            "takeout",
            "inspect",
            path_text,
            "--input-dir",
            temp.path().to_str().unwrap(),
        ],
    );
    assert!(!conflict.status.success());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let files: Vec<PathBuf> = std::fs::read_dir(temp.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(files, vec![path]);
}

#[test]
fn duplicate_central_names_and_bad_checksums_are_rejected() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("test.zip");
    zip_fixture(
        &path,
        &[
            (HISTORY, "[]"),
            ("Takeout/a.txt", "a"),
            ("Takeout/b.txt", "b"),
        ],
    );
    let mut bytes = std::fs::read(&path).unwrap();
    let needle = b"Takeout/b.txt";
    let positions: Vec<usize> = bytes
        .windows(needle.len())
        .enumerate()
        .filter_map(|(index, window)| (window == needle).then_some(index))
        .collect();
    for index in positions {
        bytes[index + 8] = b'a';
    }
    std::fs::write(&path, bytes).unwrap();
    assert!(
        inspect_archive(select(&path), &ArchiveReadLimits::default())
            .unwrap_err()
            .to_string()
            .contains("duplicate")
    );

    let mut writer = ZipWriter::new(File::create(&path).unwrap());
    writer
        .start_file(
            HISTORY,
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
    writer.write_all(RECORDS.as_bytes()).unwrap();
    writer.finish().unwrap();
    let mut bytes = std::fs::read(&path).unwrap();
    let position = bytes
        .windows(b"Watched Synthetic".len())
        .position(|window| window == b"Watched Synthetic")
        .unwrap();
    bytes[position] = b'X';
    std::fs::write(&path, bytes).unwrap();
    assert!(
        read_archive(ArchiveReadRequest {
            selected: select(&path),
            limits: ArchiveReadLimits::default()
        })
        .unwrap_err()
        .to_string()
        .contains("checksum")
    );
}

#[test]
fn empty_history_csv_still_has_stable_headers() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("test.zip");
    zip_fixture(&path, &[(HISTORY, "[]")]);
    let output = command(
        &temp,
        &[
            "takeout",
            "read",
            path.to_str().unwrap(),
            "--output-format",
            "csv",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let csv = String::from_utf8(output.stdout).unwrap();
    assert_eq!(csv.lines().count(), 1);
    assert!(csv.starts_with("\"video-id\",\"watched-at\""));
}
