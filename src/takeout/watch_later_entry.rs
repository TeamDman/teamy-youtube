use crate::takeout::YoutubeVideoId;
use chrono::DateTime;
use chrono::FixedOffset;
use eyre::WrapErr as _;

/// A single row from the Watch Later playlist CSV export.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WatchLaterEntry {
    pub video_id: YoutubeVideoId,
    pub added_at: DateTime<FixedOffset>,
}

impl WatchLaterEntry {
    /// Parse a Watch Later CSV row.
    ///
    /// # Errors
    ///
    /// Returns an error if the row is missing fields or contains invalid data.
    pub fn parse_csv_line(line_number: usize, line: &str) -> eyre::Result<Self> {
        let Some((video_id, added_at)) = line.split_once(',') else {
            eyre::bail!("row {line_number} is missing the expected comma separator");
        };

        let video_id = YoutubeVideoId::new(video_id.trim())
            .wrap_err_with(|| format!("invalid video id on row {line_number}"))?;
        let added_at = DateTime::parse_from_rfc3339(added_at.trim()).wrap_err_with(|| {
            format!(
                "invalid playlist timestamp {:?} on row {line_number}",
                added_at.trim()
            )
        })?;

        Ok(Self { video_id, added_at })
    }
}

#[cfg(test)]
mod tests {
    use super::WatchLaterEntry;

    #[test]
    fn parses_watch_later_csv_row() {
        let entry =
            WatchLaterEntry::parse_csv_line(2, "bQVXiDC5w54,2026-03-26T17:55:54+00:00").unwrap();

        assert_eq!(entry.video_id.as_str(), "bQVXiDC5w54");
        assert_eq!(entry.added_at.to_rfc3339(), "2026-03-26T17:55:54+00:00");
    }
}
