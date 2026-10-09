use crate::cli::output::CliOutput;
use arbitrary::Arbitrary;
use facet::Facet;

/// Show the cache path.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct CacheShowArgs;

#[derive(Facet, Debug)]
#[facet(rename_all = "kebab-case")]
struct CacheShowReport {
    cache_dir: String,
}

impl CacheShowArgs {
    /// # Errors
    ///
    /// This function does not return any errors.
    #[expect(clippy::unused_async)]
    pub async fn invoke(self) -> eyre::Result<CliOutput> {
        Ok(CliOutput::facet(
            CacheShowReport {
                cache_dir: crate::paths::CACHE_DIR.display().to_string(),
            },
            Some(std::time::Duration::from_secs(1)),
        ))
    }
}
