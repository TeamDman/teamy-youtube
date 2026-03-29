use arbitrary::Arbitrary;
use facet::Facet;
use figue as args;
use tracing::info;

/// Import Google Takeout playlist and history exports.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct TakeoutImportArgs {
    /// Path to the Google Takeout Watch Later playlist CSV.
    #[facet(args::named)]
    pub watch_later_csv: String,

    /// Path to the Google Takeout watch-history JSON export.
    #[facet(args::named)]
    pub watch_history_json: String,
}

impl TakeoutImportArgs {
    /// # Errors
    ///
    /// This function will return an error if either takeout file cannot be read or parsed.
    pub async fn invoke(self) -> eyre::Result<()> {
        let watch_later_csv = std::path::PathBuf::from(&self.watch_later_csv);
        let watch_history_json = std::path::PathBuf::from(&self.watch_history_json);

        info!(
            watch_later_csv = %watch_later_csv.display(),
            watch_history_json = %watch_history_json.display(),
            "importing Google Takeout files"
        );

        let watch_later_entries =
            crate::takeout::read_watch_later_entries(&watch_later_csv).await?;
        let watch_history_report =
            crate::takeout::read_watch_history_entries(&watch_history_json).await?;
        let summary = crate::takeout::ImportSummary::from_entries(
            &watch_later_entries,
            &watch_history_report.entries,
            watch_history_report.skipped_entry_count,
        );
        let import_directory = crate::takeout::persist_takeout_import(
            &crate::paths::APP_HOME,
            &watch_later_csv,
            &watch_history_json,
            &watch_later_entries,
            &watch_history_report.entries,
            &summary,
        )
        .await?;

        println!("Imported Google Takeout sources");
        println!("import-directory={}", import_directory.display());
        println!("watch-later-csv={}", watch_later_csv.display());
        println!("watch-history-json={}", watch_history_json.display());
        println!("watch-later-entries={}", summary.watch_later_entry_count);
        println!(
            "watch-later-unique-video-ids={}",
            summary.watch_later_unique_video_count
        );
        println!(
            "watch-history-entries={}",
            summary.watch_history_entry_count
        );
        println!(
            "watch-history-unique-video-ids={}",
            summary.watch_history_unique_video_count
        );
        println!(
            "watch-history-skipped-entries={}",
            summary.watch_history_skipped_entry_count
        );
        println!("overlap-video-ids={}", summary.overlap_video_count);

        Ok(())
    }
}
