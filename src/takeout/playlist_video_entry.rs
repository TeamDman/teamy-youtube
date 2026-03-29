use crate::takeout::YoutubeVideoId;
use chrono::{DateTime, FixedOffset};
use eyre::WrapErr as _;

/// A single row from a playlist CSV export.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlaylistVideoEntry {
    pub playlist_id: String,
    pub playlist_name: String,
    pub source_file: String,
    pub video_id: YoutubeVideoId,
    pub added_at: DateTime<FixedOffset>,
}

impl PlaylistVideoEntry {
    /// Parse a playlist CSV row.
    ///
    /// # Errors
    ///
    /// Returns an error if the row is missing fields or contains invalid data.
    pub fn parse_csv_line(
        line_number: usize,
        line: &str,
        playlist_id: String,
        playlist_name: String,
        source_file: String,
    ) -> eyre::Result<Self> {
        let Some((video_id, added_at)) = line.split_once(',') else {
            eyre::bail!("row {line_number} is missing the expected comma separator");
        };

        let video_id = YoutubeVideoId::new(video_id.trim().to_owned())
            .wrap_err_with(|| format!("invalid video id on row {line_number}"))?;
        let added_at = DateTime::parse_from_rfc3339(added_at.trim()).wrap_err_with(|| {
            format!(
                "invalid playlist timestamp {:?} on row {line_number}",
                added_at.trim()
            )
        })?;

        Ok(Self {
            playlist_id,
            playlist_name,
            source_file,
            video_id,
            added_at,
        })
    }
}