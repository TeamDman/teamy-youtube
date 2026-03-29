use facet::Facet;

/// A generic event file stored in the sync database.
#[derive(Clone, Debug, Facet, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct VideoEventFile {
    pub imported_at: String,
    pub source_kind: String,
    pub source_path: String,
    pub video_id: String,
    pub video_title: Option<String>,
    pub channel_name: Option<String>,
    pub event_kind: String,
    pub event_at: String,
    pub playlist_id: Option<String>,
    pub playlist_name: Option<String>,
}
