use crate::cli::output::CliOutput;
use arbitrary::Arbitrary;
use facet::Facet;

/// Delete the cache files.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct CacheCleanArgs;

#[derive(Facet, Debug)]
#[facet(rename_all = "kebab-case")]
struct CacheCleanReport {
    removed_cache_entries: usize,
}

impl CacheCleanArgs {
    /// # Errors
    ///
    /// This function will return an error if cache cleanup fails.
    #[expect(clippy::unused_async)]
    pub async fn invoke(self) -> eyre::Result<CliOutput> {
        let result = crate::paths::clean_cache()?;
        Ok(CliOutput::facet(CacheCleanReport {
            removed_cache_entries: result.entries_removed,
        }))
    }
}
