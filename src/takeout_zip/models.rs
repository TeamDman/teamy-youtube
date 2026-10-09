use facet::Facet;
use std::path::PathBuf;

/// Read-only archive discovery request; callers can inject candidates without MFT.
#[derive(Clone, Debug, Facet, PartialEq)]
pub struct ArchiveSelectionRequest {
    pub zip: Option<PathBuf>,
    pub input_dirs: Vec<PathBuf>,
    pub mft_max_age_seconds: u64,
}

/// Resource limits apply before decoding any recognized entry.
#[derive(Clone, Debug, Facet, PartialEq)]
pub struct ArchiveReadLimits {
    pub max_entries: usize,
    pub max_entry_bytes: u64,
    pub max_candidates: usize,
    pub max_directory_entries: usize,
}

impl Default for ArchiveReadLimits {
    fn default() -> Self {
        Self {
            max_entries: 100_000,
            max_entry_bytes: 256 * 1024 * 1024,
            max_candidates: 256,
            max_directory_entries: 100_000,
        }
    }
}

/// File identity observed at selection time, separate from decoded records.
/// Size and modification time detect ordinary changes, not malicious replacement.
#[derive(Clone, Debug, Facet, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct ArchiveSnapshot {
    pub path: String,
    pub bytes: u64,
    pub modified_unix_ms: u64,
}

#[derive(Clone, Debug, Facet, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct ArchiveCandidate {
    pub path: String,
    pub filename_timestamp: Option<String>,
    pub status: String,
}

#[derive(Clone, Debug, Facet, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct SelectedArchive {
    pub snapshot: ArchiveSnapshot,
    pub discovery_source: String,
    pub date_basis: String,
    pub filename_timestamp: Option<String>,
    pub candidates: Vec<ArchiveCandidate>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Facet, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct ArchiveEntryInventory {
    pub name: String,
    pub kind: String,
    pub bytes: u64,
    pub compressed_bytes: u64,
}

#[derive(Clone, Debug, Facet, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct ArchiveInspection {
    pub selected: SelectedArchive,
    pub entries: Vec<ArchiveEntryInventory>,
    pub history_entry: Option<String>,
}

/// A watch activity with source provenance. It does not imply completed viewing.
#[derive(Clone, Debug, Facet, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct ArchiveWatchRecord {
    pub video_id: String,
    pub watched_at: String,
    pub title: String,
    pub channel_name: Option<String>,
    pub channel_url: Option<String>,
    pub channel_id: Option<String>,
    pub archive_path: String,
    pub entry_name: String,
    pub record_index: usize,
}

#[derive(Clone, Debug, Facet, PartialEq)]
pub struct ArchiveReadRequest {
    pub selected: SelectedArchive,
    pub limits: ArchiveReadLimits,
}

#[derive(Clone, Debug, Facet, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct ArchiveReadResult {
    pub inspection: ArchiveInspection,
    pub parsing_mode: String,
    pub skipped_record_count: usize,
    pub records: Vec<ArchiveWatchRecord>,
}
