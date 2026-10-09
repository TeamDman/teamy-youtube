use crate::cli::output::CliOutput;
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

#[derive(Facet, Debug)]
#[facet(rename_all = "kebab-case")]
struct ApiKeySetReport {
    persisted_path: String,
}

impl ApiKeySetArgs {
    /// # Errors
    ///
    /// This function will return an error if the API key cannot be persisted.
    #[expect(clippy::unused_async)]
    pub async fn invoke(self) -> eyre::Result<CliOutput> {
        let persisted_path = crate::paths::set_youtube_api_key(&self.value)?;
        Ok(CliOutput::facet(
            ApiKeySetReport {
                persisted_path: persisted_path.display().to_string(),
            },
            Some(std::time::Duration::from_secs(1)),
        ))
    }
}
