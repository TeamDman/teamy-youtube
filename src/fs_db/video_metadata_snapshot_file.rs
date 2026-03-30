use facet::Facet;

/// A video metadata snapshot stored in the sync database.
#[derive(Clone, Debug, Facet, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct VideoMetadataSnapshotFile {
    pub fetched_at: String,
    pub source_kind: String,
    pub source_url: String,
    pub video_id: String,
    pub title: String,
    pub description: String,
    pub channel_id: String,
    pub channel_name: String,
    pub published_at: String,
    pub duration_iso8601: String,
    pub view_count: Option<u64>,
    pub like_count: Option<u64>,
    pub comment_count: Option<u64>,
    pub privacy_status: Option<String>,
}
