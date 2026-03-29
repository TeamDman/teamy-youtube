use arbitrary::Arbitrary;
use facet::Facet;

/// Show the resolved configuration values.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct ConfigShowArgs;

impl ConfigShowArgs {
    /// # Errors
    ///
    /// This function does not return any errors.
    #[expect(clippy::unused_async)]
    pub async fn invoke(self) -> eyre::Result<()> {
        println!("home={}", crate::paths::APP_HOME.display());
        println!("cache={}", crate::paths::CACHE_DIR.display());
        println!("home-env-var={}", crate::paths::APP_HOME_ENV_VAR);
        println!("cache-env-var={}", crate::paths::APP_CACHE_ENV_VAR);
        Ok(())
    }
}
