use crate::takeout::YouTubeVideoId;
use crate::youtube_api::YouTubeThumbnail;
use crate::youtube_api::YouTubeVideoFetchOutcome;
use eyre::WrapErr as _;
use facet::Facet;
use facet_json::RawJson;

/// Fetched video metadata normalized from the `YouTube` Data API.
#[derive(Clone, Debug, PartialEq)]
pub struct YouTubeFetchedVideoMetadata {
    pub raw_response_body: RawJson<'static>,
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
pub async fn fetch_video_data(
    video_id: &YouTubeVideoId,
    api_key: &str,
) -> eyre::Result<YouTubeVideoFetchOutcome> {
    let request_url = format!(
        "https://www.googleapis.com/youtube/v3/videos?part=contentDetails,id,snippet,statistics,status&id={}&key={}&hl=en",
        video_id.as_str(),
        api_key
    );
    let source_url = format!(
        "https://www.googleapis.com/youtube/v3/videos?part=contentDetails,id,snippet,statistics,status&id={}&key=[redacted]&hl=en",
        video_id.as_str(),
    );
    let response = reqwest::Client::new()
        .get(&request_url)
        .send()
        .await
        .wrap_err("failed to call YouTube Data API")?;
    if !response.status().is_success() {
        let status = response.status();
        let response_body = response
            .text()
            .await
            .wrap_err("failed reading YouTube Data API error response body")?;
        if status == reqwest::StatusCode::NOT_FOUND || status == reqwest::StatusCode::FORBIDDEN {
            return Ok(YouTubeVideoFetchOutcome::Unavailable {
                video_id: video_id.as_str().to_owned(),
                source_url,
                status_code: status.as_u16(),
                raw_response_body: RawJson::from(response_body),
            });
        }

        eyre::bail!("YouTube Data API request failed with status {status}");
    }

    let response_body = response
        .text()
        .await
        .wrap_err("failed reading YouTube Data API response body")?;
    let parsed = parse_video_response(&response_body)?;
    let Some(item) = parsed.items.into_iter().next() else {
        return Ok(YouTubeVideoFetchOutcome::Missing {
            video_id: video_id.as_str().to_owned(),
            source_url,
            raw_response_body: RawJson::from(response_body),
        });
    };

    Ok(YouTubeVideoFetchOutcome::Found(Box::new(
        YouTubeFetchedVideoMetadata {
            raw_response_body: RawJson::from(response_body),
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
        },
    )))
}

/// Fetch a single video's metadata and require a successful video payload.
///
/// # Errors
///
/// Returns an error if the API request fails or if the video is missing or unavailable.
pub async fn fetch_video_metadata(
    video_id: &YouTubeVideoId,
    api_key: &str,
) -> eyre::Result<YouTubeFetchedVideoMetadata> {
    match fetch_video_data(video_id, api_key).await? {
        YouTubeVideoFetchOutcome::Found(metadata) => Ok(*metadata),
        YouTubeVideoFetchOutcome::Missing { .. } => {
            eyre::bail!("No YouTube video found for {}", video_id.as_str())
        }
        YouTubeVideoFetchOutcome::Unavailable { status_code, .. } => eyre::bail!(
            "YouTube video {} is unavailable with status {}",
            video_id.as_str(),
            status_code
        ),
    }
}

/// Validate that a `YouTube` Data API key can successfully fetch public video metadata.
///
/// # Errors
///
/// Returns an error if the API key is invalid or the request fails.
pub async fn validate_api_key(api_key: &str) -> eyre::Result<()> {
    let validation_video_id = YouTubeVideoId::new("dQw4w9WgXcQ")?;
    let _metadata = fetch_video_metadata(&validation_video_id, api_key).await?;
    Ok(())
}

/// Extract thumbnail variants from a raw video API response body.
///
/// # Errors
///
/// Returns an error if the response body is not valid `YouTube` video API JSON.
pub fn extract_thumbnails_from_video_response(
    raw_response_body: &str,
) -> eyre::Result<Vec<YouTubeThumbnail>> {
    let parsed = parse_video_response(raw_response_body)?;
    let Some(item) = parsed.items.into_iter().next() else {
        return Ok(Vec::new());
    };

    Ok([
        ("default", item.snippet.thumbnails.default),
        ("medium", item.snippet.thumbnails.medium),
        ("high", item.snippet.thumbnails.high),
        ("standard", item.snippet.thumbnails.standard),
        ("maxres", item.snippet.thumbnails.maxres),
    ]
    .into_iter()
    .filter_map(|(name, thumbnail)| {
        thumbnail.map(|inner| YouTubeThumbnail {
            name: name.to_owned(),
            url: inner.url,
            width: inner.width,
            height: inner.height,
        })
    })
    .collect())
}

/// Extract the published-at timestamp from a raw video API response body.
///
/// # Errors
///
/// Returns an error if the response body is not valid `YouTube` video API JSON.
pub fn extract_published_at_from_video_response(
    raw_response_body: &str,
) -> eyre::Result<Option<String>> {
    let parsed = parse_video_response(raw_response_body)?;
    Ok(parsed
        .items
        .into_iter()
        .next()
        .map(|item| item.snippet.published_at))
}

fn parse_video_response(raw_response_body: &str) -> eyre::Result<YouTubeVideosResponse> {
    facet_json::from_str(raw_response_body)
        .wrap_err("failed parsing YouTube Data API response JSON")
}

fn parse_u64(value: Option<&String>) -> Option<u64> {
    value.and_then(|inner| inner.parse().ok())
}

#[derive(Debug, Facet, PartialEq)]
struct YouTubeVideosResponse {
    items: Vec<YouTubeVideoItem>,
}

#[derive(Debug, Facet, PartialEq)]
struct YouTubeVideoItem {
    id: String,
    #[facet(rename = "contentDetails")]
    content_details: YouTubeContentDetails,
    snippet: YouTubeSnippet,
    #[facet(default)]
    statistics: Option<YouTubeStatistics>,
    #[facet(default)]
    status: Option<YouTubeStatus>,
}

#[derive(Debug, Facet, PartialEq)]
struct YouTubeContentDetails {
    duration: String,
}

#[derive(Debug, Facet, PartialEq)]
#[facet(rename_all = "camelCase")]
struct YouTubeSnippet {
    published_at: String,
    channel_id: String,
    title: String,
    description: String,
    channel_title: String,
    #[facet(default)]
    thumbnails: YouTubeThumbnails,
}

#[derive(Debug, Default, Facet, PartialEq)]
struct YouTubeThumbnails {
    #[facet(default, rename = "default")]
    default: Option<YouTubeThumbnailValue>,
    #[facet(default)]
    medium: Option<YouTubeThumbnailValue>,
    #[facet(default)]
    high: Option<YouTubeThumbnailValue>,
    #[facet(default)]
    standard: Option<YouTubeThumbnailValue>,
    #[facet(default)]
    maxres: Option<YouTubeThumbnailValue>,
}

#[derive(Debug, Facet, PartialEq)]
struct YouTubeThumbnailValue {
    url: String,
    #[facet(default)]
    width: Option<u64>,
    #[facet(default)]
    height: Option<u64>,
}

#[derive(Debug, Facet, PartialEq)]
#[facet(rename_all = "camelCase")]
struct YouTubeStatistics {
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
struct YouTubeStatus {
    privacy_status: String,
}

#[cfg(test)]
mod tests {
    use super::extract_published_at_from_video_response;
    use super::extract_thumbnails_from_video_response;

    #[test]
    fn extracts_thumbnail_variants_from_response() {
        let thumbnails = extract_thumbnails_from_video_response(
            r#"{"items":[{"id":"abc123","contentDetails":{"duration":"PT1M"},"snippet":{"publishedAt":"2026-01-01T00:00:00Z","channelId":"UC123","title":"Example","description":"desc","channelTitle":"Channel","thumbnails":{"default":{"url":"https://example.invalid/default.jpg","width":120,"height":90},"high":{"url":"https://example.invalid/high.jpg","width":480,"height":360}}},"statistics":null,"status":null}]}"#,
        )
        .expect("thumbnails should parse");

        assert_eq!(thumbnails.len(), 2);
        assert_eq!(thumbnails[0].name, "default");
        assert_eq!(thumbnails[1].name, "high");
    }

    #[test]
    fn extracts_published_at_from_response() {
        let published_at = extract_published_at_from_video_response(
            r#"{"items":[{"id":"abc123","contentDetails":{"duration":"PT1M"},"snippet":{"publishedAt":"2026-01-01T00:00:00Z","channelId":"UC123","title":"Example","description":"desc","channelTitle":"Channel","thumbnails":{}},"statistics":null,"status":null}]}"#,
        )
        .expect("published-at should parse");

        assert_eq!(published_at.as_deref(), Some("2026-01-01T00:00:00Z"));
    }
}
