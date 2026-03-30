use crate::takeout::YoutubeVideoId;
use eyre::WrapErr as _;
use facet::Facet;

/// Fetched video metadata normalized from the `YouTube` Data API.
#[derive(Clone, Debug, PartialEq)]
pub struct YoutubeFetchedVideoMetadata {
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

/// Fetch a single video's metadata from the `YouTube` Data API.
///
/// # Errors
///
/// Returns an error if the API request fails, the response cannot be parsed, or no video is found.
pub async fn fetch_video_metadata(
    video_id: &YoutubeVideoId,
    api_key: &str,
) -> eyre::Result<YoutubeFetchedVideoMetadata> {
    let source_url = format!(
        "https://www.googleapis.com/youtube/v3/videos?part=contentDetails,id,snippet,statistics,status&id={}&key={}&hl=en",
        video_id.as_str(),
        api_key
    );
    let response = reqwest::Client::new()
        .get(&source_url)
        .send()
        .await
        .wrap_err("failed to call YouTube Data API")?;
    if !response.status().is_success() {
        eyre::bail!(
            "YouTube Data API request failed with status {}",
            response.status()
        );
    }

    let response_body = response
        .text()
        .await
        .wrap_err("failed reading YouTube Data API response body")?;
    let parsed: YoutubeVideosResponse = facet_json::from_str(&response_body)
        .wrap_err("failed parsing YouTube Data API response JSON")?;
    let item = parsed
        .items
        .into_iter()
        .next()
        .ok_or_else(|| eyre::eyre!("No YouTube video found for {}", video_id.as_str()))?;

    Ok(YoutubeFetchedVideoMetadata {
        source_url,
        video_id: item.id,
        title: item.snippet.title,
        description: item.snippet.description,
        channel_id: item.snippet.channel_id,
        channel_name: item.snippet.channel_title,
        published_at: item.snippet.published_at,
        duration_iso8601: item.content_details.duration,
        view_count: item
            .statistics
            .as_ref()
            .and_then(|value| parse_u64(value.views.as_ref())),
        like_count: item
            .statistics
            .as_ref()
            .and_then(|value| parse_u64(value.likes.as_ref())),
        comment_count: item
            .statistics
            .as_ref()
            .and_then(|value| parse_u64(value.comments.as_ref())),
        privacy_status: item.status.map(|value| value.privacy_status),
    })
}

/// Validate that a `YouTube` Data API key can successfully fetch public video metadata.
///
/// # Errors
///
/// Returns an error if the API key is invalid or the request fails.
pub async fn validate_api_key(api_key: &str) -> eyre::Result<()> {
    let validation_video_id = YoutubeVideoId::new("dQw4w9WgXcQ")?;
    let _metadata = fetch_video_metadata(&validation_video_id, api_key).await?;
    Ok(())
}

fn parse_u64(value: Option<&String>) -> Option<u64> {
    value.and_then(|inner| inner.parse().ok())
}

#[derive(Debug, Facet, PartialEq)]
struct YoutubeVideosResponse {
    items: Vec<YoutubeVideoItem>,
}

#[derive(Debug, Facet, PartialEq)]
struct YoutubeVideoItem {
    id: String,
    #[facet(rename = "contentDetails")]
    content_details: YoutubeContentDetails,
    snippet: YoutubeSnippet,
    #[facet(default)]
    statistics: Option<YoutubeStatistics>,
    #[facet(default)]
    status: Option<YoutubeStatus>,
}

#[derive(Debug, Facet, PartialEq)]
struct YoutubeContentDetails {
    duration: String,
}

#[derive(Debug, Facet, PartialEq)]
#[facet(rename_all = "camelCase")]
struct YoutubeSnippet {
    published_at: String,
    channel_id: String,
    title: String,
    description: String,
    channel_title: String,
}

#[derive(Debug, Facet, PartialEq)]
#[facet(rename_all = "camelCase")]
struct YoutubeStatistics {
    #[facet(rename = "viewCount")]
    #[facet(default)]
    views: Option<String>,
    #[facet(rename = "likeCount")]
    #[facet(default)]
    likes: Option<String>,
    #[facet(rename = "commentCount")]
    #[facet(default)]
    comments: Option<String>,
}

#[derive(Debug, Facet, PartialEq)]
#[facet(rename_all = "camelCase")]
struct YoutubeStatus {
    privacy_status: String,
}
