use crate::paths::APP_HOME;
use crate::paths::APP_SYNC_DIR_ENV_VAR;
use eyre::WrapErr as _;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

const SYNC_DIR_FILE_NAME: &str = "sync_dir.txt";

/// Return the configured sync directory, preferring the environment override.
///
/// # Errors
///
/// Returns an error if the persisted configuration file cannot be read.
pub fn get_sync_dir() -> eyre::Result<Option<PathBuf>> {
    if let Ok(value) = std::env::var(APP_SYNC_DIR_ENV_VAR) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            debug!(env = APP_SYNC_DIR_ENV_VAR, sync_dir = trimmed, "using sync dir from environment");
            return Ok(Some(PathBuf::from(trimmed)));
        }
    }

    let persisted_path = APP_HOME.file_path(SYNC_DIR_FILE_NAME);
    if !persisted_path.exists() {
        return Ok(None);
    }

    let persisted_value = std::fs::read_to_string(&persisted_path)
        .wrap_err_with(|| format!("failed reading {}", persisted_path.display()))?;
    let persisted_value = persisted_value.trim();
    if persisted_value.is_empty() {
        return Ok(None);
    }

    Ok(Some(PathBuf::from(persisted_value)))
}

/// Return the configured sync directory or fail with a user-facing message.
///
/// # Errors
///
/// Returns an error if the sync directory is unset or the persisted setting cannot be read.
pub fn try_get_sync_dir() -> eyre::Result<PathBuf> {
    let Some(sync_dir) = get_sync_dir()? else {
        eyre::bail!(
            "Sync directory is not set. Please set it with `teamy-youtube sync dir set <path>`"
        );
    };
    Ok(sync_dir)
}

/// Persist the sync directory for future runs.
///
/// # Errors
///
/// Returns an error if the persisted sync-dir file cannot be written.
pub fn set_sync_dir(path: &Path) -> eyre::Result<()> {
    if std::env::var(APP_SYNC_DIR_ENV_VAR).is_ok() {
        warn!(
            env = APP_SYNC_DIR_ENV_VAR,
            "{} is set and will override the persisted sync dir",
            APP_SYNC_DIR_ENV_VAR
        );
    }

    APP_HOME.ensure_dir()?;
    let persisted_path = APP_HOME.file_path(SYNC_DIR_FILE_NAME);
    std::fs::write(&persisted_path, format!("{}\n", path.display()))
        .wrap_err_with(|| format!("failed writing {}", persisted_path.display()))?;
    Ok(())
}