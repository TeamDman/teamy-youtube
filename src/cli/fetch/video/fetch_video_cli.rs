use crate::cli::output::CliOutput;
use arbitrary::Arbitrary;
use chrono::Local;
use facet::Facet;
use figue as args;

/// Fetch metadata for a specific `YouTube` video.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct FetchVideoArgs {
    /// The `YouTube` video ID or URL to fetch.
    #[facet(args::positional, proxy = crate::takeout::YouTubeVideoArgumentProxy)]
    pub video: crate::takeout::YouTubeVideoArgument<'static>,
}

#[derive(Facet, Debug)]
#[facet(rename_all = "kebab-case")]
#[repr(u8)]
enum FetchVideoResultKind {
    Found,
    Missing,
    Unavailable,
}

#[derive(Facet, Debug)]
#[facet(rename_all = "kebab-case")]
struct FetchVideoReport {
    video_id: String,
    result: FetchVideoResultKind,
    video_title: Option<String>,
    channel_id: Option<String>,
    channel_name: Option<String>,
    fetch_event: String,
    title_observation: Option<String>,
    fetch_status_code: Option<u16>,
}

impl FetchVideoArgs {
    /// # Errors
    ///
    /// This function will return an error if the sync dir is unset, the API key is missing,
    /// the video ID is invalid, the API request fails, or snapshot files cannot be written.
    pub async fn invoke(self) -> eyre::Result<CliOutput> {
        let sync_dir = crate::paths::try_get_sync_dir()?;
        let api_key = crate::paths::try_get_youtube_api_key()?;

        let video_id = self.video.video_id();
        let fetched_at = Local::now().to_rfc3339();
        let report = match crate::youtube_api::fetch_video_data(&video_id, &api_key).await? {
            crate::youtube_api::YouTubeVideoFetchOutcome::Found(metadata) => {
                let (fetch_event_path, title_observation_path) =
                    crate::fs_db::write_fetched_video_data(&sync_dir, &fetched_at, &metadata)
                        .await?;
                FetchVideoReport {
                    video_id: metadata.video_id.to_string(),
                    result: FetchVideoResultKind::Found,
                    video_title: Some(metadata.title),
                    channel_id: Some(metadata.channel_id),
                    channel_name: Some(metadata.channel_name),
                    fetch_event: fetch_event_path.display().to_string(),
                    title_observation: Some(title_observation_path.display().to_string()),
                    fetch_status_code: None,
                }
            }
            crate::youtube_api::YouTubeVideoFetchOutcome::Missing {
                video_id,
                raw_response_body,
                ..
            } => {
                let event_path = crate::fs_db::write_missing_video_data(
                    &sync_dir,
                    &fetched_at,
                    video_id,
                    "fetch_video_data_missing",
                    &raw_response_body,
                )
                .await?;
                FetchVideoReport {
                    video_id: video_id.to_string(),
                    result: FetchVideoResultKind::Missing,
                    video_title: None,
                    channel_id: None,
                    channel_name: None,
                    fetch_event: event_path.display().to_string(),
                    title_observation: None,
                    fetch_status_code: None,
                }
            }
            crate::youtube_api::YouTubeVideoFetchOutcome::Unavailable {
                video_id,
                status_code,
                raw_response_body,
                ..
            } => {
                let event_path = crate::fs_db::write_missing_video_data(
                    &sync_dir,
                    &fetched_at,
                    video_id,
                    "fetch_video_data_unavailable",
                    &raw_response_body,
                )
                .await?;
                FetchVideoReport {
                    video_id: video_id.to_string(),
                    result: FetchVideoResultKind::Unavailable,
                    video_title: None,
                    channel_id: None,
                    channel_name: None,
                    fetch_event: event_path.display().to_string(),
                    title_observation: None,
                    fetch_status_code: Some(status_code),
                }
            }
        };
        Ok(CliOutput::facet(
            report,
            Some(std::time::Duration::from_secs(35)), /* One HTTP metadata request plus local snapshot writes. */
        ))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn cli_parses_video_url_into_typed_argument() {
        let args: super::FetchVideoArgs =
            figue::from_slice(&["https://www.youtube.com/watch?v=bQVXiDC5w54"]).unwrap();

        assert_eq!(args.video.video_id().as_str(), "bQVXiDC5w54");
    }
}
