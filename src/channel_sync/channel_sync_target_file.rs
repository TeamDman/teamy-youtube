use facet::Facet;

/// Persisted configuration for a channel that should be synced into a preferred download directory.
#[derive(Clone, Debug, Eq, Facet, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct ChannelSyncTargetFile {
    pub added_at: String,
    pub requested_input: String,
    pub source_url: String,
    pub channel_id: String,
    pub channel_name: String,
    pub uploader_id: Option<String>,
    pub uploader_url: Option<String>,
    pub channel_url: Option<String>,
    pub preferred_download_dir: String,
}

/// Stored summary of one channel-video discovery pass.
#[derive(Clone, Debug, Eq, Facet, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct ChannelDiscoveryEventFile {
    pub discovered_at: String,
    pub source_url: String,
    pub channel_id: String,
    pub channel_name: String,
    pub discovered_video_count: usize,
    pub videos: Vec<ChannelDiscoveryEventVideo>,
}

/// One discovered channel video recorded in a discovery event.
#[derive(Clone, Debug, Eq, Facet, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct ChannelDiscoveryEventVideo {
    pub video_id: String,
    pub video_url: String,
    pub video_title: Option<String>,
}
