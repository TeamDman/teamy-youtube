use crate::cli::output::CliOutput;
use crate::cli::takeout::TakeoutZipArgs;
use crate::takeout_zip::ArchiveReadRequest;
use crate::takeout_zip::read_archive;
use crate::takeout_zip::select_archive;
use arbitrary::Arbitrary;
use facet::Facet;

#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct TakeoutReadArgs {
    #[facet(flatten, default)]
    pub archive: TakeoutZipArgs,
}

impl TakeoutReadArgs {
    /// # Errors
    /// Returns an error if ZIP selection or bounded watch-history decoding fails.
    pub fn invoke(self) -> eyre::Result<CliOutput> {
        let (request, limits) = self.archive.prepare()?;
        let selected = select_archive(&request, &limits)?;
        let report = read_archive(ArchiveReadRequest { selected, limits })?;
        // Five seconds selection, one second per MiB decoded, 2 ms per inventory/record item.
        let history_bytes = report
            .inspection
            .entries
            .iter()
            .filter(|entry| Some(&entry.name) == report.inspection.history_entry.as_ref())
            .fold(0_u64, |total, entry| total.saturating_add(entry.bytes));
        let budget = crate::elapsed_duration::budget(
            5000,
            usize::try_from(history_bytes / (1024 * 1024)).unwrap_or(usize::MAX),
            1000,
        )
        .saturating_add(crate::elapsed_duration::budget(
            0,
            report
                .inspection
                .entries
                .len()
                .saturating_add(report.records.len())
                .saturating_add(report.skipped_record_count),
            2,
        ));
        Ok(CliOutput::facet_rows(
            report,
            |result| result.records.clone(),
            Some(budget),
        ))
    }
}
