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
        let metadata = crate::youtube_api::fetch_video_metadata(&video_id, &api_key).await?;
        let fetched_at = Local::now().to_rfc3339();
        let (video_snapshot_path, channel_snapshot_path) =
            crate::fs_db::write_fetched_video_metadata(&sync_dir, &fetched_at, &metadata).await?;

        println!("video-id={}", metadata.video_id);
        println!("video-title={}", metadata.title);
        println!("channel-id={}", metadata.channel_id);
        println!("channel-name={}", metadata.channel_name);
        println!("video-snapshot={}", video_snapshot_path.display());
        println!("channel-snapshot={}", channel_snapshot_path.display());
        Ok(())
    }
}
