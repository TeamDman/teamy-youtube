use facet::Facet;

/// A channel metadata snapshot stored in the sync database.
#[derive(Clone, Debug, Facet, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct ChannelMetadataSnapshotFile {
    pub fetched_at: String,
    pub source_kind: String,
    pub source_url: String,
    pub channel_id: String,
    pub channel_name: String,
}
