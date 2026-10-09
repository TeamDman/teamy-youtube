use crate::cli::output::CliOutput;
use arbitrary::Arbitrary;
use facet::Facet;

/// Validate that a configured `YouTube` API key is usable.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct ApiKeyValidateArgs;

#[derive(Facet, Debug)]
#[facet(rename_all = "kebab-case")]
struct ApiKeyValidateReport {
    valid: bool,
}

impl ApiKeyValidateArgs {
    /// # Errors
    ///
    /// This function will return an error if the API key is unset or invalid.
    pub async fn invoke(self) -> eyre::Result<CliOutput> {
        let api_key = crate::paths::try_get_youtube_api_key()?;
        crate::youtube_api::validate_api_key(&api_key).await?;
        Ok(CliOutput::facet(
            ApiKeyValidateReport { valid: true },
            Some(std::time::Duration::from_secs(35)), /* One HTTP validation request plus setup. */
        ))
    }
}
