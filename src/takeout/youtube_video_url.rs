use arbitrary::Arbitrary;
use facet::Facet;

/// A validated `YouTube` watch or share URL that resolves to a canonical video id.
#[derive(Clone, Debug, Eq, Facet, PartialEq)]
#[facet(proxy = YouTubeVideoUrlProxy)]
pub struct YouTubeVideoUrl {
    raw: String,
    video_id: YouTubeVideoId,
}

use crate::takeout::YouTubeVideoId;

impl YouTubeVideoUrl {
    /// # Errors
    ///
    /// Returns an error if the provided string is not a supported `YouTube` video URL.
    pub fn try_new(value: &str) -> eyre::Result<Self> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            eyre::bail!("video url cannot be empty");
        }

        let url = reqwest::Url::parse(trimmed)
            .map_err(|error| eyre::eyre!("invalid YouTube video url `{trimmed}`: {error}"))?;
        let Some(video_id) = extract_video_id(&url)? else {
            eyre::bail!(
                "unsupported YouTube video url `{trimmed}`; expected a watch or youtu.be URL"
            );
        };

        Ok(Self {
            raw: trimmed.to_owned(),
            video_id,
        })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    #[must_use]
    pub fn video_id(&self) -> YouTubeVideoId {
        self.video_id
    }

    #[must_use]
    pub fn into_video_id(self) -> YouTubeVideoId {
        self.video_id
    }
}

impl Arbitrary<'_> for YouTubeVideoUrl {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let video_id = YouTubeVideoId::arbitrary(u)?;
        let raw = if u.arbitrary::<bool>()? {
            format!("https://www.youtube.com/watch?v={video_id}")
        } else {
            format!("https://youtu.be/{video_id}")
        };
        Ok(Self { raw, video_id })
    }
}

impl std::fmt::Display for YouTubeVideoUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.raw)
    }
}

impl std::str::FromStr for YouTubeVideoUrl {
    type Err = eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_new(s)
    }
}

/// Transparent string proxy for `YouTubeVideoUrl` in `Facet` surfaces.
#[derive(Clone, Debug, Eq, Facet, PartialEq)]
#[facet(transparent)]
pub struct YouTubeVideoUrlProxy(pub String);

impl TryFrom<YouTubeVideoUrlProxy> for YouTubeVideoUrl {
    type Error = eyre::Report;

    fn try_from(value: YouTubeVideoUrlProxy) -> Result<Self, Self::Error> {
        value.0.parse()
    }
}

impl TryFrom<&YouTubeVideoUrl> for YouTubeVideoUrlProxy {
    type Error = std::convert::Infallible;

    fn try_from(value: &YouTubeVideoUrl) -> Result<Self, Self::Error> {
        Ok(Self(value.to_string()))
    }
}

fn extract_video_id(url: &reqwest::Url) -> eyre::Result<Option<YouTubeVideoId>> {
    match url.scheme() {
        "http" | "https" => {}
        scheme => eyre::bail!("unsupported URL scheme `{scheme}`"),
    }

    let Some(host) = url.host_str() else {
        return Ok(None);
    };

    match host {
        "youtu.be" => extract_youtu_be_video_id(url),
        "youtube.com" | "www.youtube.com" | "music.youtube.com" => {
            extract_watch_video_id(url)
        }
        _ => Ok(None),
    }
}

fn extract_youtu_be_video_id(url: &reqwest::Url) -> eyre::Result<Option<YouTubeVideoId>> {
    let mut segments = match url.path_segments() {
        Some(segments) => segments,
        None => return Ok(None),
    };
    let Some(segment) = segments.next() else {
        return Ok(None);
    };
    if segments.next().is_some() {
        return Ok(None);
    }

    Ok(Some(YouTubeVideoId::try_new(segment)?))
}

fn extract_watch_video_id(url: &reqwest::Url) -> eyre::Result<Option<YouTubeVideoId>> {
    if url.path() != "/watch" {
        return Ok(None);
    }

    let Some((_, value)) = url.query_pairs().find(|(key, _)| key == "v") else {
        return Ok(None);
    };

    Ok(Some(YouTubeVideoId::try_new(value.as_ref())?))
}

#[cfg(test)]
mod tests {
    use super::YouTubeVideoUrl;

    #[test]
    fn parses_watch_url_query_param() {
        let video_url =
            YouTubeVideoUrl::try_new("https://www.youtube.com/watch?v=bQVXiDC5w54&list=WL")
                .unwrap();

        assert_eq!(video_url.video_id().as_str(), "bQVXiDC5w54");
    }

    #[test]
    fn parses_youtu_be_url() {
        let video_url = YouTubeVideoUrl::try_new("https://youtu.be/bQVXiDC5w54?t=31").unwrap();

        assert_eq!(video_url.video_id().as_str(), "bQVXiDC5w54");
    }

    #[test]
    fn rejects_channel_url() {
        assert!(YouTubeVideoUrl::try_new("https://www.youtube.com/channel/abc123def45").is_err());
    }

    #[test]
    fn rejects_playlist_url_without_video_id() {
        assert!(YouTubeVideoUrl::try_new("https://www.youtube.com/playlist?list=PL1234567890").is_err());
    }
}
