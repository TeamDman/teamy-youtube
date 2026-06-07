use arbitrary::Arbitrary;
use eyre::WrapErr as _;
use facet::Facet;
use std::borrow::Borrow;
use std::str::FromStr;

const YOUTUBE_VIDEO_ID_LENGTH: usize = 11;

/// A canonical `YouTube` video identifier.
#[derive(Clone, Copy, Debug, Eq, Facet, Hash, Ord, PartialEq, PartialOrd)]
#[facet(proxy = YouTubeVideoIdProxy)]
pub struct YouTubeVideoId([u8; YOUTUBE_VIDEO_ID_LENGTH]);

impl YouTubeVideoId {
    /// # Errors
    ///
    /// Returns an error if the provided string is not a canonical `YouTube` video id.
    pub fn try_new(value: &str) -> eyre::Result<Self> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            eyre::bail!("video id cannot be empty");
        }
        if trimmed.len() != YOUTUBE_VIDEO_ID_LENGTH {
            eyre::bail!(
                "video id must be exactly {YOUTUBE_VIDEO_ID_LENGTH} characters long"
            );
        }
        if !trimmed
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            eyre::bail!(
                "video id must contain only ASCII letters, digits, '-' or '_'"
            );
        }

        let bytes = <[u8; YOUTUBE_VIDEO_ID_LENGTH]>::try_from(trimmed.as_bytes())
            .wrap_err("video id should have exactly 11 bytes after validation")?;
        Ok(Self(bytes))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).expect("validated ASCII video ids are valid UTF-8")
    }
}

impl Arbitrary<'_> for YouTubeVideoId {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        const ALPHABET: &[u8] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

        let mut bytes = [b'A'; YOUTUBE_VIDEO_ID_LENGTH];
        for byte in &mut bytes {
            let index = u.int_in_range(0..=ALPHABET.len() - 1)?;
            *byte = ALPHABET[index];
        }
        Ok(Self(bytes))
    }
}

impl std::fmt::Display for YouTubeVideoId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for YouTubeVideoId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for YouTubeVideoId {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for YouTubeVideoId {
    type Err = eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_new(s)
    }
}

/// Transparent string proxy for `YouTubeVideoId` in `Facet` surfaces.
#[derive(Clone, Debug, Eq, Facet, PartialEq)]
#[facet(transparent)]
pub struct YouTubeVideoIdProxy(pub String);

impl TryFrom<YouTubeVideoIdProxy> for YouTubeVideoId {
    type Error = eyre::Report;

    fn try_from(value: YouTubeVideoIdProxy) -> Result<Self, Self::Error> {
        value.0.parse()
    }
}

impl TryFrom<&YouTubeVideoId> for YouTubeVideoIdProxy {
    type Error = std::convert::Infallible;

    fn try_from(value: &YouTubeVideoId) -> Result<Self, Self::Error> {
        Ok(Self(value.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::YouTubeVideoId;

    #[test]
    fn accepts_valid_canonical_video_id() {
        let video_id = YouTubeVideoId::try_new("bQVXiDC5w54").unwrap();

        assert_eq!(video_id.as_str(), "bQVXiDC5w54");
    }

    #[test]
    fn rejects_empty_video_id() {
        assert!(YouTubeVideoId::try_new("   ").is_err());
    }

    #[test]
    fn rejects_wrong_length_video_id() {
        assert!(YouTubeVideoId::try_new("abc123").is_err());
        assert!(YouTubeVideoId::try_new("abcdefghijkl").is_err());
    }

    #[test]
    fn rejects_invalid_characters() {
        assert!(YouTubeVideoId::try_new("abc123def4!").is_err());
    }
}
