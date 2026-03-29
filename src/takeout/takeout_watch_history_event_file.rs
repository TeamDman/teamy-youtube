use facet::Facet;

/// Persisted watch-history event file from a takeout import.
#[derive(Clone, Debug, Facet, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct TakeoutWatchHistoryEventFile {
    pub imported_at: String,
    pub source_file: String,
    pub video_id: String,
    pub title: String,
    pub channel_name: Option<String>,
    pub watched_at: String,
}

impl TakeoutWatchHistoryEventFile {
    #[must_use]
    pub fn new(
        imported_at: String,
        source_file: String,
        video_id: String,
        title: String,
        channel_name: Option<String>,
        watched_at: String,
    ) -> Self {
        Self {
            imported_at,
            source_file,
            video_id,
            title,
            channel_name,
            watched_at,
        }
    }
}
