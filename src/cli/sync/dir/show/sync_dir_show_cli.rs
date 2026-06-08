use crate::cli::output::CliOutput;
use arbitrary::Arbitrary;
use facet::Facet;

/// Show the current sync directory.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct SyncDirShowArgs;

#[derive(Facet, Debug)]
#[facet(rename_all = "kebab-case")]
struct SyncDirShowReport {
    is_set: bool,
    sync_dir: Option<String>,
}

impl SyncDirShowArgs {
    /// # Errors
    ///
    /// This function will return an error if reading the persisted sync dir fails.
    #[expect(clippy::unused_async)]
    pub async fn invoke(self) -> eyre::Result<CliOutput> {
        let sync_dir = crate::paths::get_sync_dir()?;
        Ok(CliOutput::facet(SyncDirShowReport {
            is_set: sync_dir.is_some(),
            sync_dir: sync_dir.map(|value| value.display().to_string()),
        }))
    }
}
