use arbitrary::Arbitrary;
use facet::Facet;
use figue as args;
use std::path::PathBuf;

/// Persist the sync directory used for the filesystem database.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct SyncDirSetArgs {
    /// The directory to use for the persisted sync database.
    #[facet(args::positional)]
    pub path: String,
}

impl SyncDirSetArgs {
    /// # Errors
    ///
    /// This function will return an error if the path cannot be normalized or persisted.
    #[expect(clippy::unused_async)]
    pub async fn invoke(self) -> eyre::Result<()> {
        let expanded = expand_user_home(&self.path)?;
        let sync_dir = if expanded.is_absolute() {
            expanded
        } else {
            std::env::current_dir()?.join(expanded)
        };

        std::fs::create_dir_all(&sync_dir)?;
        let sync_dir = dunce::canonicalize(&sync_dir)?;
        crate::paths::set_sync_dir(&sync_dir)?;
        println!("{}", sync_dir.display());
        Ok(())
    }
}

fn expand_user_home(value: &str) -> eyre::Result<PathBuf> {
    if value == "~" || value.starts_with("~/") || value.starts_with("~\\") {
        let home = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)
            .ok_or_else(|| eyre::eyre!("Could not determine user home directory"))?;
        let suffix = value
            .trim_start_matches('~')
            .trim_start_matches(['/', '\\']);
        if suffix.is_empty() {
            return Ok(home);
        }
        return Ok(home.join(suffix));
    }

    Ok(PathBuf::from(value))
}
