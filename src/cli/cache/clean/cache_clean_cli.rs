use arbitrary::Arbitrary;
use facet::Facet;

/// Delete the cache files.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct CacheCleanArgs;

impl CacheCleanArgs {
    /// # Errors
    ///
    /// This function will return an error if cache cleanup fails.
    #[expect(clippy::unused_async)]
    pub async fn invoke(self) -> eyre::Result<()> {
        let result = crate::paths::clean_cache()?;
        println!("removed-cache-entries={}", result.entries_removed);
        Ok(())
    }
}