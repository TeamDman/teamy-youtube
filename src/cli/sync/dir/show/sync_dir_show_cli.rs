use arbitrary::Arbitrary;
use facet::Facet;

/// Show the current sync directory.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct SyncDirShowArgs;

impl SyncDirShowArgs {
    /// # Errors
    ///
    /// This function will return an error if reading the persisted sync dir fails.
    #[expect(clippy::unused_async)]
    pub async fn invoke(self) -> eyre::Result<()> {
        match crate::paths::get_sync_dir()? {
            Some(sync_dir) => println!("{}", sync_dir.display()),
            None => println!("sync-dir-not-set"),
        }
        Ok(())
    }
}
