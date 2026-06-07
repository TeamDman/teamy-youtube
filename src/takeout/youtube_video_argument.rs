use crate::takeout::YouTubeVideoId;
use crate::takeout::YouTubeVideoUrl;
use arbitrary::Arbitrary;
use eyre::bail;
use facet::Facet;
use std::str::FromStr;

/// `YouTube` video input accepted by user-facing boundaries.
#[derive(Debug, Clone, Facet, PartialEq, Eq)]
#[facet(opaque)]
#[repr(u8)]
pub enum YouTubeVideoArgument<'a> {
    Id(YouTubeVideoId),
    IdRef(&'a YouTubeVideoId),
    Url(YouTubeVideoUrl),
    UrlRef(&'a YouTubeVideoUrl),
}

/// Transparent string proxy for `YouTubeVideoArgument` in `Facet`/`figue` surfaces.
#[derive(Clone, Debug, Eq, Facet, PartialEq)]
#[facet(transparent)]
pub struct YouTubeVideoArgumentProxy(pub String);

impl Arbitrary<'_> for YouTubeVideoArgument<'static> {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        if u.arbitrary::<bool>()? {
            Ok(Self::Id(YouTubeVideoId::arbitrary(u)?))
        } else {
            Ok(Self::Url(YouTubeVideoUrl::arbitrary(u)?))
        }
    }
}

impl std::fmt::Display for YouTubeVideoArgument<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Id(value) => value.fmt(f),
            Self::IdRef(value) => value.fmt(f),
            Self::Url(value) => value.fmt(f),
            Self::UrlRef(value) => value.fmt(f),
        }
    }
}

impl From<YouTubeVideoId> for YouTubeVideoArgument<'_> {
    fn from(value: YouTubeVideoId) -> Self {
        Self::Id(value)
    }
}

impl<'a> From<&'a YouTubeVideoId> for YouTubeVideoArgument<'a> {
    fn from(value: &'a YouTubeVideoId) -> Self {
        Self::IdRef(value)
    }
}

impl From<YouTubeVideoUrl> for YouTubeVideoArgument<'_> {
    fn from(value: YouTubeVideoUrl) -> Self {
        Self::Url(value)
    }
}

impl<'a> From<&'a YouTubeVideoUrl> for YouTubeVideoArgument<'a> {
    fn from(value: &'a YouTubeVideoUrl) -> Self {
        Self::UrlRef(value)
    }
}

impl YouTubeVideoArgument<'_> {
    pub fn into_owned(self) -> YouTubeVideoArgument<'static> {
        match self {
            Self::Id(value) => YouTubeVideoArgument::Id(value),
            Self::IdRef(value) => YouTubeVideoArgument::Id(*value),
            Self::Url(value) => YouTubeVideoArgument::Url(value),
            Self::UrlRef(value) => YouTubeVideoArgument::Url(value.clone()),
        }
    }

    #[must_use]
    pub fn video_id(&self) -> YouTubeVideoId {
        match self {
            Self::Id(value) => *value,
            Self::IdRef(value) => **value,
            Self::Url(value) => value.video_id(),
            Self::UrlRef(value) => value.video_id(),
        }
    }
}

impl FromStr for YouTubeVideoArgument<'static> {
    type Err = eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(video_id) = s.parse::<YouTubeVideoId>() {
            Ok(Self::Id(video_id))
        } else if let Ok(video_url) = s.parse::<YouTubeVideoUrl>() {
            Ok(Self::Url(video_url))
        } else {
            bail!("`{s}` is not a valid YouTube video id or supported video url")
        }
    }
}

impl TryFrom<YouTubeVideoArgumentProxy> for YouTubeVideoArgument<'static> {
    type Error = eyre::Report;

    fn try_from(value: YouTubeVideoArgumentProxy) -> Result<Self, Self::Error> {
        value.0.parse()
    }
}

impl TryFrom<&YouTubeVideoArgument<'_>> for YouTubeVideoArgumentProxy {
    type Error = std::convert::Infallible;

    fn try_from(value: &YouTubeVideoArgument<'_>) -> Result<Self, Self::Error> {
        Ok(Self(value.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::YouTubeVideoArgument;

    #[test]
    fn parses_canonical_id_argument() {
        let argument: YouTubeVideoArgument<'static> = "bQVXiDC5w54".parse().unwrap();

        assert_eq!(argument.video_id().as_str(), "bQVXiDC5w54");
    }

    #[test]
    fn parses_supported_url_argument() {
        let argument: YouTubeVideoArgument<'static> =
            "https://www.youtube.com/watch?v=bQVXiDC5w54".parse().unwrap();

        assert_eq!(argument.video_id().as_str(), "bQVXiDC5w54");
    }

    #[test]
    fn rejects_malformed_argument() {
        assert!("not a video".parse::<YouTubeVideoArgument<'static>>().is_err());
    }
}
