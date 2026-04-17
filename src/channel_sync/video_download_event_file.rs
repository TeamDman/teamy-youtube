use facet::Facet;

/// Persisted request to download a video into a preferred directory.
#[derive(Clone, Debug, Eq, Facet, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct VideoDownloadRequestEventFile {
    pub requested_at: String,
    pub source_kind: String,
    pub source_url: String,
    pub channel_id: String,
    pub channel_name: String,
    pub video_id: String,
    pub video_url: String,
    pub video_title: Option<String>,
    pub preferred_download_dir: String,
}

/// Persisted completion record for a successfully downloaded video.
#[derive(Clone, Debug, Eq, Facet, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct VideoDownloadCompletedEventFile {
    pub downloaded_at: String,
    pub source_kind: String,
    pub source_url: String,
    pub channel_id: String,
    pub channel_name: String,
    pub video_id: String,
    pub video_url: String,
    pub video_title: Option<String>,
    pub preferred_download_dir: String,
    pub media_path: String,
    pub media_bytes: u64,
}

/// Persisted terminal failure for one video download attempt.
#[derive(Clone, Debug, Eq, Facet, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct VideoDownloadFailedEventFile {
    pub failed_at: String,
    pub source_kind: String,
    pub source_url: String,
    pub channel_id: String,
    pub channel_name: String,
    pub video_id: String,
    pub video_url: String,
    pub video_title: Option<String>,
    pub preferred_download_dir: String,
    pub exit_code: Option<i32>,
    pub stdout_excerpt: Option<String>,
    pub stderr_excerpt: Option<String>,
}
