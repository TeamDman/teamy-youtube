use facet::Facet;

/// An observation that a refreshed thumbnail was byte-for-byte unchanged.
#[derive(Clone, Debug, Facet, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct ThumbnailUnchangedEventFile {
    pub observed_at: String,
    pub video_id: String,
    pub thumbnail_size: String,
    pub width: Option<u64>,
    pub height: Option<u64>,
    pub source_url: String,
    pub compared_asset_path: String,
}
