use facet::Facet;

/// Manifest for a single persisted takeout import run.
#[derive(Debug, Facet, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct TakeoutImportManifest {
    pub import_id: String,
    pub imported_at: String,
    pub watch_later_csv: String,
    pub watch_history_json: String,
    pub watch_later_entry_count: usize,
    pub watch_later_unique_video_count: usize,
    pub watch_history_entry_count: usize,
    pub watch_history_unique_video_count: usize,
    pub watch_history_skipped_entry_count: usize,
    pub overlap_video_count: usize,
}

impl TakeoutImportManifest {
    #[expect(
        clippy::too_many_arguments,
        reason = "manifest construction mirrors persisted fields"
    )]
    #[must_use]
    pub fn new(
        import_id: String,
        imported_at: String,
        watch_later_csv: String,
        watch_history_json: String,
        watch_later_entry_count: usize,
        watch_later_unique_video_count: usize,
        watch_history_entry_count: usize,
        watch_history_unique_video_count: usize,
        watch_history_skipped_entry_count: usize,
        overlap_video_count: usize,
    ) -> Self {
        Self {
            import_id,
            imported_at,
            watch_later_csv,
            watch_history_json,
            watch_later_entry_count,
            watch_later_unique_video_count,
            watch_history_entry_count,
            watch_history_unique_video_count,
            watch_history_skipped_entry_count,
            overlap_video_count,
        }
    }
}
