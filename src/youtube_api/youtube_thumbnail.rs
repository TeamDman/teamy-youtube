/// A thumbnail variant returned by the `YouTube` Data API.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct YouTubeThumbnail {
    pub name: String,
    pub url: String,
    pub width: Option<u64>,
    pub height: Option<u64>,
}

impl YouTubeThumbnail {
    // yt[sync.thumbnails.size-keyed-assets]
    /// Return the canonical size key used for thumbnail file naming.
    #[must_use]
    pub fn size_key(&self) -> String {
        match (self.width, self.height) {
            (Some(width), Some(height)) => format!("{width}x{height}"),
            _ => self.name.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::YouTubeThumbnail;

    #[test]
    fn prefers_dimensions_for_size_key() {
        let thumbnail = YouTubeThumbnail {
            name: "default".to_owned(),
            url: "https://example.invalid/default.jpg".to_owned(),
            width: Some(120),
            height: Some(90),
        };

        assert_eq!(thumbnail.size_key(), "120x90");
    }

    #[test]
    fn falls_back_to_name_when_dimensions_are_missing() {
        let thumbnail = YouTubeThumbnail {
            name: "default".to_owned(),
            url: "https://example.invalid/default.jpg".to_owned(),
            width: None,
            height: None,
        };

        assert_eq!(thumbnail.size_key(), "default");
    }
}
