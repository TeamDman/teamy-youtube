mod app_home;
mod cache;
mod sync_dir;

pub use app_home::*;
pub use cache::*;
pub use sync_dir::*;

pub const APP_HOME_ENV_VAR: &str = "TEAMY_YOUTUBE_HOME";
pub const APP_HOME_DIR_NAME: &str = "teamy-youtube";

pub const APP_CACHE_ENV_VAR: &str = "TEAMY_YOUTUBE_CACHE_DIR";
pub const APP_CACHE_DIR_NAME: &str = "teamy-youtube";

pub const APP_SYNC_DIR_ENV_VAR: &str = "TEAMY_YOUTUBE_SYNC_DIR";
