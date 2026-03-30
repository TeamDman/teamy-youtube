use crate::takeout::WatchHistoryEntry;
use crate::takeout::WatchLaterEntry;
use crate::takeout::YouTubeVideoId;
use std::collections::BTreeSet;

/// High-level counts for a takeout import run.
#[derive(Debug, PartialEq)]
pub struct ImportSummary {
    pub watch_later_entry_count: usize,
    pub watch_later_unique_video_count: usize,
    pub watch_history_entry_count: usize,
    pub watch_history_unique_video_count: usize,
    pub watch_history_skipped_entry_count: usize,
    pub overlap_video_count: usize,
}

impl ImportSummary {
    #[must_use]
    pub fn from_entries(
        watch_later_entries: &[WatchLaterEntry],
        watch_history_entries: &[WatchHistoryEntry],
        watch_history_skipped_entry_count: usize,
    ) -> Self {
        let watch_later_video_ids = watch_later_entries
            .iter()
            .map(|entry| entry.video_id.clone())
            .collect::<BTreeSet<YouTubeVideoId>>();
        let watch_history_video_ids = watch_history_entries
            .iter()
            .map(|entry| entry.video_id.clone())
            .collect::<BTreeSet<YouTubeVideoId>>();
        let overlap_video_count = watch_later_video_ids
            .intersection(&watch_history_video_ids)
            .count();

        Self {
            watch_later_entry_count: watch_later_entries.len(),
            watch_later_unique_video_count: watch_later_video_ids.len(),
            watch_history_entry_count: watch_history_entries.len(),
            watch_history_unique_video_count: watch_history_video_ids.len(),
            watch_history_skipped_entry_count,
            overlap_video_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ImportSummary;
    use crate::takeout::WatchHistoryEntry;
    use crate::takeout::WatchLaterEntry;
    use crate::takeout::YouTubeVideoId;
    use chrono::DateTime;
    use chrono::FixedOffset;

    #[test]
    fn summarizes_overlap_by_video_id() {
        let watch_later_entries = vec![
            WatchLaterEntry {
                video_id: YouTubeVideoId::new("shared-video").unwrap(),
                added_at: DateTime::parse_from_rfc3339("2026-03-26T17:55:54+00:00").unwrap(),
            },
            WatchLaterEntry {
                video_id: YouTubeVideoId::new("watch-later-only").unwrap(),
                added_at: DateTime::parse_from_rfc3339("2026-03-26T17:56:54+00:00").unwrap(),
            },
        ];
        let watch_history_entries = vec![
            WatchHistoryEntry {
                video_id: YouTubeVideoId::new("shared-video").unwrap(),
                title: "Shared video".to_owned(),
                channel_name: Some("Example channel".to_owned()),
                watched_at: DateTime::<FixedOffset>::parse_from_rfc3339(
                    "2026-03-26T18:55:54+00:00",
                )
                .unwrap(),
            },
            WatchHistoryEntry {
                video_id: YouTubeVideoId::new("history-only").unwrap(),
                title: "History only".to_owned(),
                channel_name: None,
                watched_at: DateTime::<FixedOffset>::parse_from_rfc3339(
                    "2026-03-26T19:55:54+00:00",
                )
                .unwrap(),
            },
        ];

        let summary = ImportSummary::from_entries(&watch_later_entries, &watch_history_entries, 3);

        assert_eq!(summary.watch_later_entry_count, 2);
        assert_eq!(summary.watch_later_unique_video_count, 2);
        assert_eq!(summary.watch_history_entry_count, 2);
        assert_eq!(summary.watch_history_unique_video_count, 2);
        assert_eq!(summary.watch_history_skipped_entry_count, 3);
        assert_eq!(summary.overlap_video_count, 1);
    }
}
