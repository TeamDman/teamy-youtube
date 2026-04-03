use crate::youtube_api::YouTubeFetchedVideoMetadata;
use facet_json::RawJson;

/// The terminal outcome of fetching a video's data from the `YouTube` Data API.
#[derive(Clone, Debug, PartialEq)]
pub enum YouTubeVideoFetchOutcome {
    Found(Box<YouTubeFetchedVideoMetadata>),
    Missing {
        video_id: String,
        source_url: String,
        raw_response_body: RawJson<'static>,
    },
    Unavailable {
        video_id: String,
        source_url: String,
        status_code: u16,
        raw_response_body: RawJson<'static>,
    },
}
