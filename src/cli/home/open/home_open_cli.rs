use crate::cli::output::CliOutput;
use arbitrary::Arbitrary;
use eyre::WrapErr as _;
use facet::Facet;

/// Open the home path in the platform file manager.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct HomeOpenArgs;

#[derive(Facet, Debug)]
#[facet(rename_all = "kebab-case")]
struct HomeOpenReport {
    home_dir: String,
    opened: bool,
}

impl HomeOpenArgs {
    /// # Errors
    ///
    /// This function will return an error if the home directory cannot be created
    /// or the file manager cannot be launched.
    #[expect(clippy::unused_async)]
    pub async fn invoke(self) -> eyre::Result<CliOutput> {
        crate::paths::APP_HOME.ensure_dir()?;
        open::that_detached(crate::paths::APP_HOME.0.as_path()).wrap_err_with(|| {
            format!(
                "Failed to open {} in file manager",
                crate::paths::APP_HOME.0.display()
            )
        })?;
        Ok(CliOutput::facet(HomeOpenReport {
            home_dir: crate::paths::APP_HOME.display().to_string(),
            opened: true,
        }))
    }
}
