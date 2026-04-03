use arbitrary::Arbitrary;
use facet::Facet;
use figue as args;

/// Sync generic event data between Postgres and the filesystem database.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct SyncRunPostgresArgs {
    /// Optional Postgres connection string. Falls back to env vars when omitted.
    #[facet(args::named)]
    pub database_url: Option<String>,
}

impl SyncRunPostgresArgs {
    /// # Errors
    ///
    /// This function will return an error if the sync dir is unset, the database URL is
    /// unavailable, the Postgres sync fails, or the filesystem database cannot be read/written.
    pub async fn invoke(self) -> eyre::Result<()> {
        let sync_dir = crate::paths::try_get_sync_dir()?;
        let database_url =
            crate::postgres_sync::resolve_database_url(self.database_url.as_deref())?;
        let summary = crate::postgres_sync::sync_postgres(&sync_dir, &database_url).await?;

        println!("sync-dir={}", sync_dir.display());
        println!("postgres-fsdb-event-count={}", summary.fsdb_event_count);
        println!(
            "postgres-db-upserted-event-count={}",
            summary.postgres_upserted_event_count
        );
        println!("postgres-db-event-count={}", summary.postgres_event_count);
        println!(
            "postgres-fsdb-written-event-file-count={}",
            summary.fsdb_written_event_file_count
        );
        println!(
            "postgres-fsdb-existing-event-file-count={}",
            summary.fsdb_existing_event_file_count
        );
        Ok(())
    }
}
