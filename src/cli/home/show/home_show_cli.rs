use arbitrary::Arbitrary;
use facet::Facet;

/// Show the home path.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct HomeShowArgs;

impl HomeShowArgs {
    /// # Errors
    ///
    /// This function does not return any errors.
    #[expect(clippy::unused_async)]
    pub async fn invoke(self) -> eyre::Result<()> {
        println!("{}", crate::paths::APP_HOME.display());
        Ok(())
    }
}