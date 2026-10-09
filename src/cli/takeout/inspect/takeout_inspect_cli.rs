use crate::cli::output::CliOutput;
use crate::cli::takeout::TakeoutZipArgs;
use crate::takeout_zip::inspect_archive;
use crate::takeout_zip::select_archive;
use arbitrary::Arbitrary;
use facet::Facet;

#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct TakeoutInspectArgs {
    #[facet(flatten, default)]
    pub archive: TakeoutZipArgs,
}

impl TakeoutInspectArgs {
    /// # Errors
    /// Returns an error if ZIP selection or entry inspection fails.
    pub fn invoke(self) -> eyre::Result<CliOutput> {
        let (request, limits) = self.archive.prepare()?;
        let selected = select_archive(&request, &limits)?;
        let report = inspect_archive(selected, &limits)?;
        // Central-directory inventory: five seconds selection plus 2 ms per ZIP entry.
        let budget = crate::elapsed_duration::budget(5000, report.entries.len(), 2);
        Ok(CliOutput::facet_rows(
            report,
            |inspection| inspection.entries.clone(),
            Some(budget),
        ))
    }
}
