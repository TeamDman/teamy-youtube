/// High-level counts for a bidirectional Postgres/filesystem sync run.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PostgresSyncSummary {
    pub fsdb_event_count: usize,
    pub postgres_upserted_event_count: usize,
    pub postgres_event_count: usize,
    pub fsdb_written_event_file_count: usize,
    pub fsdb_existing_event_file_count: usize,
}
