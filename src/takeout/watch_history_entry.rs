use crate::takeout::YoutubeVideoId;
use crate::takeout::raw_watch_history_entry::RawWatchHistoryEntry;
use chrono::DateTime;
use chrono::FixedOffset;
use eyre::WrapErr as _;

/// A watch-history entry that resolves to a concrete `YouTube` video.
#[derive(Clone, Debug, PartialEq)]
pub struct WatchHistoryEntry {
    pub video_id: YoutubeVideoId,
    pub title: String,
    pub channel_name: Option<String>,
    pub watched_at: DateTime<FixedOffset>,
}

impl WatchHistoryEntry {
    /// Convert a raw takeout entry into a typed watch entry.
    ///
    /// Returns `Ok(None)` for rows that do not point to a supported watch URL.
    ///
    /// # Errors
    ///
    /// Returns an error if the row looks like a watch entry but contains malformed data.
    pub fn try_from_raw(raw: RawWatchHistoryEntry) -> eyre::Result<Option<Self>> {
        let Some(title_url) = raw.title_url else {
            return Ok(None);
        };

        let Some(video_id) = YoutubeVideoId::from_watch_url(&title_url) else {
            return Ok(None);
        };

        let watched_at = DateTime::parse_from_rfc3339(&raw.time)
            .wrap_err_with(|| format!("invalid RFC3339 timestamp {:?}", raw.time))?;
        let channel_name = raw
            .subtitles
            .into_iter()
            .next()
            .map(|subtitle| subtitle.name);

        Ok(Some(Self {
            video_id,
            title: raw.title,
            channel_name,
            watched_at,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::WatchHistoryEntry;
    use crate::takeout::raw_watch_history_entry::RawWatchHistoryEntry;
    use crate::takeout::raw_watch_history_subtitle::RawWatchHistorySubtitle;

    #[test]
    fn parses_supported_watch_url() {
        let raw = RawWatchHistoryEntry {
            header: Some("YouTube".to_owned()),
            title: "Watched Test Video".to_owned(),
            title_url: Some("https://www.youtube.com/watch?v=bQVXiDC5w54".to_owned()),
            subtitles: vec![RawWatchHistorySubtitle {
                name: "Example channel".to_owned(),
                url: None,
            }],
            time: "2026-03-26T17:55:54+00:00".to_owned(),
        };

        let entry = WatchHistoryEntry::try_from_raw(raw).unwrap().unwrap();

        assert_eq!(entry.video_id.as_str(), "bQVXiDC5w54");
        assert_eq!(entry.channel_name.as_deref(), Some("Example channel"));
    }

    #[test]
    fn skips_non_watch_entries() {
        let raw = RawWatchHistoryEntry {
            header: Some("YouTube".to_owned()),
            title: "Visited channel".to_owned(),
            title_url: Some("https://www.youtube.com/channel/abc123".to_owned()),
            subtitles: Vec::new(),
            time: "2026-03-26T17:55:54+00:00".to_owned(),
        };

        let entry = WatchHistoryEntry::try_from_raw(raw).unwrap();

        assert!(entry.is_none());
    }
}
