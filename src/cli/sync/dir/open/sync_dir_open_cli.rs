use crate::cli::output::CliOutput;
use arbitrary::Arbitrary;
use eyre::WrapErr as _;
use facet::Facet;

/// Open the sync directory in the platform file manager.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct SyncDirOpenArgs;

#[derive(Facet, Debug)]
#[facet(rename_all = "kebab-case")]
struct SyncDirOpenReport {
    sync_dir: String,
    opened: bool,
}

impl SyncDirOpenArgs {
    /// # Errors
    ///
    /// This function will return an error if the sync directory is unset,
    /// cannot be created, or the file manager cannot be launched.
    #[expect(clippy::unused_async)]
    pub async fn invoke(self) -> eyre::Result<CliOutput> {
        let sync_dir = crate::paths::try_get_sync_dir()?;
        std::fs::create_dir_all(&sync_dir)?;
        open::that_detached(sync_dir.as_path())
            .wrap_err_with(|| format!("Failed to open {} in file manager", sync_dir.display()))?;
        Ok(CliOutput::facet(
            SyncDirOpenReport {
                sync_dir: sync_dir.display().to_string(),
                opened: true,
            },
            Some(std::time::Duration::from_secs(3)),
        ))
    }
}
