/// High-level counts for a sync run that writes the filesystem database.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SyncDatabaseSummary {
    pub unique_video_count: usize,
    pub unique_playlist_count: usize,
    pub playlist_event_count: usize,
    pub watch_event_count: usize,
    pub written_event_file_count: usize,
    pub existing_event_file_count: usize,
}