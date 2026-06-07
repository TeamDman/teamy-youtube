use crate::takeout::YouTubeVideoId;
use crate::takeout::YouTubeVideoUrl;
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
    pub video_id: YouTubeVideoId,
    pub video_url: YouTubeVideoUrl,
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
    pub video_id: YouTubeVideoId,
    pub video_url: YouTubeVideoUrl,
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
    pub video_id: YouTubeVideoId,
    pub video_url: YouTubeVideoUrl,
    pub video_title: Option<String>,
    pub preferred_download_dir: String,
    pub exit_code: Option<i32>,
    pub stdout_excerpt: Option<String>,
    pub stderr_excerpt: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::VideoDownloadRequestEventFile;
    use crate::takeout::YouTubeVideoId;
    use crate::takeout::YouTubeVideoUrl;

    #[test]
    fn request_event_roundtrips_with_string_json_fields() {
        let event = VideoDownloadRequestEventFile {
            requested_at: "2026-06-07T16:17:05.523601400-04:00".to_owned(),
            source_kind: "channel-sync".to_owned(),
            source_url: "https://www.youtube.com/@example".to_owned(),
            channel_id: "UC123456789".to_owned(),
            channel_name: "Example Channel".to_owned(),
            video_id: YouTubeVideoId::try_new("tCUBfHz3UUA").expect("valid video id"),
            video_url: YouTubeVideoUrl::try_new("https://www.youtube.com/watch?v=tCUBfHz3UUA")
                .expect("valid video url"),
            video_title: Some("Example Video".to_owned()),
            preferred_download_dir: "videos/example".to_owned(),
        };

        let json = facet_json::to_string_pretty(&event).expect("event should serialize");
        assert!(json.contains(r#""video-id": "tCUBfHz3UUA""#));
        assert!(json.contains(r#""video-url": "https://www.youtube.com/watch?v=tCUBfHz3UUA""#));

        let parsed: VideoDownloadRequestEventFile =
            facet_json::from_str(&json).expect("event should deserialize");
        assert_eq!(parsed, event);
    }
}
