use arbitrary::Arbitrary;
use facet::Facet;
use figue as args;

/// Persist a `YouTube` API key in the application home directory.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct ApiKeySetArgs {
    /// The API key value to persist.
    #[facet(args::positional)]
    pub value: String,
}

impl ApiKeySetArgs {
    /// # Errors
    ///
    /// This function will return an error if the API key cannot be persisted.
    #[expect(clippy::unused_async)]
    pub async fn invoke(self) -> eyre::Result<()> {
        let persisted_path = crate::paths::set_youtube_api_key(&self.value)?;
        println!(
            "Persisted the provided YouTube API key to {}",
            persisted_path.display()
        );
        Ok(())
    }
}
