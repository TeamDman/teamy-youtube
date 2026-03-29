/// A concrete `YouTube` video identifier.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct YoutubeVideoId(String);

impl YoutubeVideoId {
    /// # Errors
    ///
    /// Returns an error if the provided string is empty.
    pub fn new(value: &str) -> eyre::Result<Self> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            eyre::bail!("video id cannot be empty");
        }

        Ok(Self(trimmed.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn from_watch_url(url: &str) -> Option<Self> {
        Self::from_youtu_be_url(url).or_else(|| Self::from_youtube_watch_url(url))
    }

    fn from_youtu_be_url(url: &str) -> Option<Self> {
        let stripped = url
            .strip_prefix("https://youtu.be/")
            .or_else(|| url.strip_prefix("http://youtu.be/"))?;
        let video_id = stripped
            .split(['?', '#', '&', '/'])
            .next()
            .unwrap_or_default()
            .trim();
        Self::new(video_id).ok()
    }

    fn from_youtube_watch_url(url: &str) -> Option<Self> {
        let supported_prefixes = [
            "https://www.youtube.com/watch?",
            "http://www.youtube.com/watch?",
            "https://youtube.com/watch?",
            "http://youtube.com/watch?",
            "https://music.youtube.com/watch?",
            "http://music.youtube.com/watch?",
        ];
        let query = supported_prefixes
            .iter()
            .find_map(|prefix| url.strip_prefix(prefix))?;

        for pair in query.split('&') {
            let (key, value) = pair.split_once('=')?;
            if key == "v" {
                return Self::new(value).ok();
            }
        }

        None
    }
}

impl std::fmt::Display for YoutubeVideoId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::YoutubeVideoId;

    #[test]
    fn parses_watch_url_query_param() {
        let video_id =
            YoutubeVideoId::from_watch_url("https://www.youtube.com/watch?v=bQVXiDC5w54&list=WL")
                .unwrap();

        assert_eq!(video_id.as_str(), "bQVXiDC5w54");
    }

    #[test]
    fn parses_youtu_be_url() {
        let video_id = YoutubeVideoId::from_watch_url("https://youtu.be/bQVXiDC5w54?t=31").unwrap();

        assert_eq!(video_id.as_str(), "bQVXiDC5w54");
    }
}
