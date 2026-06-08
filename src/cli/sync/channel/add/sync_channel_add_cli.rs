use crate::cli::output::CliOutput;
use arbitrary::Arbitrary;
use facet::Facet;
use figue as args;
use std::path::PathBuf;

/// Track a channel and remember the preferred directory for future downloads.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct SyncChannelAddArgs {
    /// A `YouTube` channel URL or `@handle`.
    #[facet(args::positional)]
    pub channel: String,

    /// Preferred directory for downloaded channel media.
    #[facet(args::positional)]
    pub preferred_download_dir: String,
}

#[derive(Facet, Debug)]
#[facet(rename_all = "kebab-case")]
struct SyncChannelAddReport {
    sync_dir: String,
    channel_id: String,
    channel_name: String,
    channel_source_url: String,
    preferred_download_dir: String,
    channel_target_file: String,
}

impl SyncChannelAddArgs {
    /// # Errors
    ///
    /// This function will return an error if the sync dir is unset, the preferred directory
    /// cannot be resolved, or yt-dlp cannot resolve the channel metadata.
    pub async fn invoke(self) -> eyre::Result<CliOutput> {
        let sync_dir = crate::paths::try_get_sync_dir()?;
        let preferred_download_dir = resolve_download_dir(&self.preferred_download_dir)?;
        std::fs::create_dir_all(&preferred_download_dir)?;
        let preferred_download_dir = dunce::canonicalize(&preferred_download_dir)?;

        let added = crate::channel_sync::add_channel_sync_target(
            &sync_dir,
            &self.channel,
            &preferred_download_dir,
        )
        .await?;

        Ok(CliOutput::facet(SyncChannelAddReport {
            sync_dir: sync_dir.display().to_string(),
            channel_id: added.target.channel_id,
            channel_name: added.target.channel_name,
            channel_source_url: added.target.source_url,
            preferred_download_dir: added.target.preferred_download_dir,
            channel_target_file: added.target_path.display().to_string(),
        }))
    }
}

fn resolve_download_dir(value: &str) -> eyre::Result<PathBuf> {
    let expanded = expand_user_home(value)?;
    if expanded.is_absolute() {
        return Ok(expanded);
    }

    Ok(std::env::current_dir()?.join(expanded))
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
