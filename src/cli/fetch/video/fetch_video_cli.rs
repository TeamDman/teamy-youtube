use arbitrary::Arbitrary;
use chrono::Local;
use facet::Facet;
use figue as args;

/// Fetch metadata for a specific `YouTube` video.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct FetchVideoArgs {
    /// The `YouTube` video ID to fetch.
    #[facet(args::positional)]
    pub video_id: String,
}

impl FetchVideoArgs {
    /// # Errors
    ///
    /// This function will return an error if the sync dir is unset, the API key is missing,
    /// the video ID is invalid, the API request fails, or snapshot files cannot be written.
    pub async fn invoke(self) -> eyre::Result<()> {
        let sync_dir = crate::paths::try_get_sync_dir()?;
        let api_key = crate::paths::try_get_youtube_api_key()?;

        let video_id = crate::takeout::YouTubeVideoId::new(&self.video_id)?;
        let fetched_at = Local::now().to_rfc3339();
        match crate::youtube_api::fetch_video_data(&video_id, &api_key).await? {
            crate::youtube_api::YouTubeVideoFetchOutcome::Found(metadata) => {
                let (fetch_event_path, title_observation_path) =
                    crate::fs_db::write_fetched_video_data(&sync_dir, &fetched_at, &metadata)
                        .await?;

                println!("video-id={}", metadata.video_id);
                println!("video-title={}", metadata.title);
                println!("channel-id={}", metadata.channel_id);
                println!("channel-name={}", metadata.channel_name);
                println!("fetch-result=found");
                println!("fetch-event={}", fetch_event_path.display());
                println!("title-observation={}", title_observation_path.display());
            }
            crate::youtube_api::YouTubeVideoFetchOutcome::Missing {
                video_id,
                raw_response_body,
                ..
            } => {
                let event_path = crate::fs_db::write_missing_video_data(
                    &sync_dir,
                    &fetched_at,
                    &video_id,
                    "fetch_video_data_missing",
                    &raw_response_body,
                )
                .await?;
                println!("video-id={video_id}");
                println!("fetch-result=missing");
                println!("fetch-event={}", event_path.display());
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
                    &video_id,
                    "fetch_video_data_unavailable",
                    &raw_response_body,
                )
                .await?;
                println!("video-id={video_id}");
                println!("fetch-result=unavailable");
                println!("fetch-status-code={status_code}");
                println!("fetch-event={}", event_path.display());
            }
        }
        Ok(())
    }
}
