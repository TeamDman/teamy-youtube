use facet::Facet;

/// An observation that a thumbnail URL was unavailable when checked.
#[derive(Clone, Debug, Facet, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct ThumbnailUnavailableEventFile {
    pub observed_at: String,
    pub video_id: String,
    pub thumbnail_size: String,
    pub source_url: String,
    pub status_code: u16,
}
