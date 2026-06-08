use crate::cli::output::CliOutput;
use arbitrary::Arbitrary;
use facet::Facet;

/// Show the home path.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct HomeShowArgs;

#[derive(Facet, Debug)]
#[facet(rename_all = "kebab-case")]
struct HomeShowReport {
    home_dir: String,
}

impl HomeShowArgs {
    /// # Errors
    ///
    /// This function does not return any errors.
    #[expect(clippy::unused_async)]
    pub async fn invoke(self) -> eyre::Result<CliOutput> {
        Ok(CliOutput::facet(HomeShowReport {
            home_dir: crate::paths::APP_HOME.display().to_string(),
        }))
    }
}
