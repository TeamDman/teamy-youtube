use super::ArchiveCandidate;
use super::ArchiveReadLimits;
use super::ArchiveSelectionRequest;
use super::SelectedArchive;
use super::inspect_archive;
use super::snapshot_archive;
use chrono::NaiveDateTime;
use eyre::Context;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Duration;
use std::time::SystemTime;

/// Resolve an explicit ZIP, configured roots, or read-only published MFT candidates.
/// # Errors
/// Returns an error for conflicting selectors, stale/unavailable MFT, ambiguous or absent matches.
pub fn select_archive(
    request: &ArchiveSelectionRequest,
    limits: &ArchiveReadLimits,
) -> eyre::Result<SelectedArchive> {
    if let Some(path) = &request.zip {
        eyre::ensure!(
            request.input_dirs.is_empty(),
            "An explicit ZIP cannot be combined with --input-dir"
        );
        return Ok(SelectedArchive {
            snapshot: snapshot_archive(path)?,
            discovery_source: "explicit-path".to_owned(),
            date_basis: "explicit selection; no date ranking".to_owned(),
            filename_timestamp: filename_timestamp(path),
            candidates: Vec::new(),
            warnings: Vec::new(),
        });
    }
    if !request.input_dirs.is_empty() {
        let mut candidates = BTreeSet::new();
        let mut directories = request.input_dirs.clone();
        let mut visited = BTreeSet::new();
        let mut entry_count = 0usize;
        while let Some(directory) = directories.pop() {
            let directory = dunce::canonicalize(&directory)
                .wrap_err("Cannot resolve Takeout discovery directory")?;
            eyre::ensure!(
                directory.is_dir(),
                "Takeout discovery root must be a directory"
            );
            if !visited.insert(directory.clone()) {
                continue;
            }
            for entry in std::fs::read_dir(directory)? {
                entry_count += 1;
                eyre::ensure!(
                    entry_count <= limits.max_directory_entries,
                    "Directory discovery entry limit exceeded; use a narrower --input-dir or explicit ZIP"
                );
                let entry = entry?;
                let file_type = entry.file_type()?;
                if file_type.is_symlink() {
                    continue;
                }
                if file_type.is_dir() {
                    directories.push(entry.path());
                } else if file_type.is_file() && is_zip_path(&entry.path()) {
                    candidates.insert(entry.path());
                    eyre::ensure!(
                        candidates.len() <= limits.max_candidates,
                        "ZIP candidate limit exceeded; narrow --input-dir or select a ZIP explicitly"
                    );
                }
            }
        }
        return select_archive_from_candidates(
            candidates.into_iter().collect(),
            "input-directories",
            Vec::new(),
            limits,
        );
    }

    teamy_mft::status::TeamyMftStatus::load_all_drives()
        .and_then(|status| status.assert_query_ready_not_older_than(Duration::from_secs(request.mft_max_age_seconds), SystemTime::now()))
        .wrap_err("Automatic ZIP selection requires a fresh published MFT index; use an explicit ZIP or --input-dir. This command does not rebuild or refresh MFT")?;
    let rows = crate::mft_query::collect_rows(&teamy_mft::cli::command::query::QueryArgs {
        plan: teamy_mft::query::QueryPlan {
            limit: limits
                .max_candidates
                .checked_add(1)
                .ok_or_else(|| eyre::eyre!("Candidate limit is too large"))?
                .into(),
            ..teamy_mft::query::QueryPlan::parse_inputs(&["takeout .zip".to_owned()])?
        },
        ..Default::default()
    })
    .wrap_err("Cannot query published MFT for Takeout ZIPs; use --input-dir or an explicit ZIP")?;
    eyre::ensure!(
        rows.len() <= limits.max_candidates,
        "MFT candidate limit exceeded; narrow discovery with --input-dir or select a ZIP explicitly"
    );
    select_archive_from_candidates(
        rows.into_iter().map(|row| PathBuf::from(row.path)).filter(|path| is_zip_path(path)).collect(),
        "published-mft-index",
        vec!["Candidates reflect published MFT paths and filter rules; recent or excluded ZIPs may be absent. No index changes or daemon operations were requested.".to_owned()],
        limits,
    )
}

/// Pure selection entrypoint for SDKs/tests with externally discovered candidate paths.
/// Eligibility is verified from the central directory; no history bytes are decoded.
/// # Errors
/// Returns an error if no dated `YouTube` JSON archive is eligible or the newest date ties.
pub fn select_archive_from_candidates(
    paths: Vec<PathBuf>,
    source: &str,
    warnings: Vec<String>,
    limits: &ArchiveReadLimits,
) -> eyre::Result<SelectedArchive> {
    eyre::ensure!(
        paths.len() <= limits.max_candidates,
        "ZIP candidate limit exceeded"
    );
    let mut seen = BTreeSet::new();
    let mut candidates = Vec::new();
    let mut eligible = Vec::new();
    for path in paths.into_iter().collect::<BTreeSet<_>>() {
        let timestamp = filename_timestamp(&path);
        let mut candidate = ArchiveCandidate {
            path: path.display().to_string(),
            filename_timestamp: timestamp.clone(),
            status: "ignored-nonstandard-takeout-filename".to_owned(),
        };
        if let Some(timestamp) = timestamp {
            if let Ok(snapshot) = snapshot_archive(&path) {
                if !seen.insert(snapshot.path.clone()) {
                    continue;
                }
                candidate.path.clone_from(&snapshot.path);
                let selected = SelectedArchive {
                    snapshot,
                    discovery_source: source.to_owned(),
                    date_basis: "newest UTC timestamp in takeout-YYYYMMDDTHHMMSSZ-<part>.zip filename; mtime is not a date fallback".to_owned(),
                    filename_timestamp: Some(timestamp.clone()),
                    candidates: Vec::new(),
                    warnings: Vec::new(),
                };
                match inspect_archive(selected.clone(), limits) {
                    Ok(inspection) if inspection.history_entry.is_some() => {
                        "eligible-youtube-watch-history-json".clone_into(&mut candidate.status);
                        eligible.push((timestamp, selected));
                    }
                    Ok(_) => {
                        "ignored-no-youtube-watch-history-json".clone_into(&mut candidate.status);
                    }
                    Err(_) => {
                        "rejected-invalid-unsafe-ambiguous-or-over-limit-zip"
                            .clone_into(&mut candidate.status);
                    }
                }
            } else {
                "rejected-unreadable-path".clone_into(&mut candidate.status);
            }
        }
        candidates.push(candidate);
    }
    candidates.sort_by(|left, right| left.path.cmp(&right.path));
    eligible.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.snapshot.path.cmp(&right.1.snapshot.path))
    });
    let Some((newest, mut selected)) = eligible.pop() else {
        let reasons = candidates
            .iter()
            .map(|candidate| format!("{}: {}", candidate.path, candidate.status))
            .collect::<Vec<_>>()
            .join("; ");
        eyre::bail!(
            "No eligible dated Takeout ZIP containing YouTube watch-history.json. Use an explicit ZIP for renamed exports. Candidates: {reasons}"
        );
    };
    let ties = eligible
        .iter()
        .filter(|(timestamp, _)| *timestamp == newest)
        .map(|(_, candidate)| candidate.snapshot.path.as_str())
        .collect::<Vec<_>>();
    eyre::ensure!(
        ties.is_empty(),
        "Ambiguous newest Takeout timestamp {newest}: {} and {}. Select one ZIP explicitly; split exports are not merged",
        selected.snapshot.path,
        ties.join(", ")
    );
    selected.candidates = candidates;
    selected.warnings = warnings;
    Ok(selected)
}

fn is_zip_path(path: &std::path::Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
}

fn filename_timestamp(path: &std::path::Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    let name = name
        .strip_suffix(".zip")
        .or_else(|| name.strip_suffix(".ZIP"))?;
    let remainder = name.strip_prefix("takeout-")?;
    let (timestamp, part) = remainder.split_once('-')?;
    if timestamp.len() != 16 || part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    NaiveDateTime::parse_from_str(timestamp, "%Y%m%dT%H%M%SZ")
        .ok()
        .map(|date| date.format("%Y-%m-%dT%H:%M:%SZ").to_string())
}
