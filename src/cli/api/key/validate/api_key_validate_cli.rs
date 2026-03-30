use arbitrary::Arbitrary;
use facet::Facet;

/// Validate that a configured `YouTube` API key is usable.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct ApiKeyValidateArgs;

impl ApiKeyValidateArgs {
    /// # Errors
    ///
    /// This function will return an error if the API key is unset or invalid.
    pub async fn invoke(self) -> eyre::Result<()> {
        let api_key = crate::paths::try_get_youtube_api_key()?;
        crate::youtube_api::validate_api_key(&api_key).await?;
        println!("The configured YouTube API key is valid!");
        Ok(())
    }
}
