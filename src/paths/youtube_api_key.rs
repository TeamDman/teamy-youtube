use crate::paths::APP_HOME;
use crate::paths::YOUTUBE_API_KEY_ENV_VAR;
use eyre::WrapErr as _;
use std::path::PathBuf;
use tracing::debug;

const YOUTUBE_API_KEY_FILE_NAME: &str = "youtube_api_key.txt";

/// Return the configured `YouTube` API key, preferring the environment variable.
///
/// # Errors
///
/// Returns an error if the persisted key file cannot be read.
pub fn get_youtube_api_key() -> eyre::Result<Option<String>> {
    get_youtube_api_key_for_home(&APP_HOME)
}

/// Return the configured `YouTube` API key or fail with a user-facing message.
///
/// # Errors
///
/// Returns an error if the API key is unset or the persisted key file cannot be read.
pub fn try_get_youtube_api_key() -> eyre::Result<String> {
    let Some(api_key) = get_youtube_api_key()? else {
        eyre::bail!(
            "YouTube API key is not set. Please set it with `teamy-youtube api key set <value>` or by setting YOUTUBE_API_KEY"
        );
    };
    Ok(api_key)
}

/// Persist the `YouTube` API key for future runs.
///
/// # Errors
///
/// Returns an error if the API key is empty or the persisted file cannot be written.
pub fn set_youtube_api_key(value: &str) -> eyre::Result<PathBuf> {
    set_youtube_api_key_for_home(&APP_HOME, value)
}

fn get_youtube_api_key_for_home(app_home: &crate::paths::AppHome) -> eyre::Result<Option<String>> {
    if let Ok(value) = std::env::var(YOUTUBE_API_KEY_ENV_VAR) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            debug!(
                env = YOUTUBE_API_KEY_ENV_VAR,
                "using YouTube API key from environment"
            );
            return Ok(Some(trimmed.to_owned()));
        }
    }

    let persisted_path = app_home.file_path(YOUTUBE_API_KEY_FILE_NAME);
    if !persisted_path.exists() {
        return Ok(None);
    }

    let persisted_value = std::fs::read_to_string(&persisted_path)
        .wrap_err_with(|| format!("failed reading {}", persisted_path.display()))?;
    let persisted_value = persisted_value.trim();
    if persisted_value.is_empty() {
        return Ok(None);
    }

    Ok(Some(persisted_value.to_owned()))
}

fn set_youtube_api_key_for_home(
    app_home: &crate::paths::AppHome,
    value: &str,
) -> eyre::Result<PathBuf> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        eyre::bail!("YouTube API key cannot be empty");
    }

    app_home.ensure_dir()?;
    let persisted_path = app_home.file_path(YOUTUBE_API_KEY_FILE_NAME);
    std::fs::write(&persisted_path, format!("{trimmed}\n"))
        .wrap_err_with(|| format!("failed writing {}", persisted_path.display()))?;
    Ok(persisted_path)
}

#[cfg(test)]
mod tests {
    use super::YOUTUBE_API_KEY_FILE_NAME;
    use super::get_youtube_api_key_for_home;
    use super::set_youtube_api_key_for_home;
    use tempfile::TempDir;

    #[test]
    fn reads_persisted_api_key_from_home() {
        let temp_dir = TempDir::new().expect("temp dir should be created");
        let app_home = crate::paths::AppHome(temp_dir.path().to_path_buf());

        let persisted_path =
            set_youtube_api_key_for_home(&app_home, "abc123").expect("api key should persist");

        let api_key = get_youtube_api_key_for_home(&app_home).expect("api key should read");

        assert_eq!(
            persisted_path,
            app_home.file_path(YOUTUBE_API_KEY_FILE_NAME)
        );
        assert_eq!(api_key.as_deref(), Some("abc123"));
    }

    #[test]
    fn rejects_empty_api_key() {
        let temp_dir = TempDir::new().expect("temp dir should be created");
        let app_home = crate::paths::AppHome(temp_dir.path().to_path_buf());

        let error =
            set_youtube_api_key_for_home(&app_home, "   ").expect_err("empty key should fail");

        assert!(error.to_string().contains("cannot be empty"));
    }
}
