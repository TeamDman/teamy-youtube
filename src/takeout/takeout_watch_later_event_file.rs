use facet::Facet;

/// Persisted Watch Later playlist membership event from a takeout import.
#[derive(Clone, Debug, Facet, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct TakeoutWatchLaterEventFile {
    pub imported_at: String,
    pub source_file: String,
    pub video_id: String,
    pub playlist_id: String,
    pub added_at: String,
}

impl TakeoutWatchLaterEventFile {
    #[must_use]
    pub fn new(
        imported_at: String,
        source_file: String,
        video_id: String,
        playlist_id: String,
        added_at: String,
    ) -> Self {
        Self {
            imported_at,
            source_file,
            video_id,
            playlist_id,
            added_at,
        }
    }
}
