use arbitrary::Arbitrary;
use facet::Facet;

/// Initialize the filesystem-backed home directory.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct ConfigInitArgs;

impl ConfigInitArgs {
    /// # Errors
    ///
    /// This function will return an error if the home or cache directories cannot be created.
    #[expect(clippy::unused_async)]
    pub async fn invoke(self) -> eyre::Result<()> {
        crate::paths::APP_HOME.ensure_dir()?;
        std::fs::create_dir_all(crate::paths::CACHE_DIR.0.as_path())?;

        println!("Initialized home at {}", crate::paths::APP_HOME.display());
        println!("Initialized cache at {}", crate::paths::CACHE_DIR.display());
        Ok(())
    }
}
