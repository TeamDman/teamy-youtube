use arbitrary::Arbitrary;
use facet::Facet;

/// Import previously collected Postgres-backed YouTube data.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct ImportPostgresArgs;

impl ImportPostgresArgs {
    /// # Errors
    ///
    /// This function does not currently return any errors.
    #[expect(clippy::unused_async)]
    pub async fn invoke(self) -> eyre::Result<()> {
        println!("TODO: import videos, watch history, and channel metadata from a Postgres export");
        Ok(())
    }
}
