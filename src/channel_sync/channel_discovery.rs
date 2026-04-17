use crate::takeout::YouTubeVideoId;
use eyre::WrapErr as _;
use facet::Facet;
use tokio::process::Command;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredChannel {
    pub source_url: String,
    pub channel_id: String,
    pub channel_name: String,
    pub uploader_id: Option<String>,
    pub uploader_url: Option<String>,
    pub channel_url: Option<String>,
    pub entries: Vec<DiscoveredChannelVideo>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredChannelVideo {
    pub video_id: YouTubeVideoId,
    pub video_url: String,
    pub video_title: Option<String>,
}

#[derive(Clone, Debug, Facet, PartialEq)]
struct YtDlpChannelPlaylist {
    id: Option<String>,
    channel: Option<String>,
    channel_id: Option<String>,
    title: Option<String>,
    uploader: Option<String>,
    uploader_id: Option<String>,
    uploader_url: Option<String>,
    channel_url: Option<String>,
    webpage_url: Option<String>,
    original_url: Option<String>,
    entries: Vec<YtDlpChannelVideoEntry>,
}

#[derive(Clone, Debug, Facet, PartialEq)]
struct YtDlpChannelVideoEntry {
    id: String,
    url: Option<String>,
    title: Option<String>,
}

pub async fn discover_channel(
    requested_input: &str,
    playlist_end: Option<usize>,
) -> eyre::Result<DiscoveredChannel> {
    let normalized_input = normalize_channel_input(requested_input)?;
    let raw = run_yt_dlp_channel_dump(&normalized_input, playlist_end).await?;

    let channel_id = raw
        .channel_id
        .clone()
        .or(raw.id.clone())
        .ok_or_else(|| eyre::eyre!("yt-dlp channel discovery did not return a channel id"))?;
    let channel_name = raw
        .channel
        .clone()
        .or(raw.uploader.clone())
        .or(raw.title.clone())
        .ok_or_else(|| eyre::eyre!("yt-dlp channel discovery did not return a channel name"))?;

    let mut entries = Vec::new();
    for entry in raw.entries {
        let video_id = YouTubeVideoId::new(&entry.id)?;
        let video_url = entry
            .url
            .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={}", video_id.as_str()));
        let video_title = entry.title.and_then(non_empty_owned);
        entries.push(DiscoveredChannelVideo {
            video_id,
            video_url,
            video_title,
        });
    }

    Ok(DiscoveredChannel {
        source_url: raw
            .webpage_url
            .clone()
            .or(raw.original_url.clone())
            .unwrap_or(normalized_input),
        channel_id,
        channel_name,
        uploader_id: raw.uploader_id,
        uploader_url: raw.uploader_url,
        channel_url: raw.channel_url,
        entries,
    })
}

fn normalize_channel_input(value: &str) -> eyre::Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        eyre::bail!("channel input cannot be empty");
    }

    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return Ok(trimmed.to_owned());
    }
    if trimmed.starts_with("www.youtube.com/") || trimmed.starts_with("youtube.com/") {
        return Ok(format!("https://{trimmed}"));
    }
    if trimmed.starts_with('@') {
        return Ok(format!("https://www.youtube.com/{trimmed}/videos"));
    }

    eyre::bail!(
        "unsupported channel input `{trimmed}`; expected a full YouTube channel URL or @handle"
    );
}

async fn run_yt_dlp_channel_dump(
    normalized_input: &str,
    playlist_end: Option<usize>,
) -> eyre::Result<YtDlpChannelPlaylist> {
    let mut command = Command::new("yt-dlp");
    command.arg("--flat-playlist");
    if let Some(playlist_end) = playlist_end {
        command.arg("--playlist-end").arg(playlist_end.to_string());
    }
    command.arg("--dump-single-json").arg(normalized_input);

    let output = command
        .output()
        .await
        .wrap_err("failed launching yt-dlp for channel discovery")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        eyre::bail!(
            "yt-dlp channel discovery failed for {normalized_input}: {}",
            if stderr.is_empty() {
                format!("exit code {:?}", output.status.code())
            } else {
                stderr
            }
        );
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| eyre::eyre!("yt-dlp channel discovery emitted non-utf8 JSON: {error}"))?;
    facet_json::from_str(stdout.trim()).wrap_err("failed parsing yt-dlp channel discovery JSON")
}

fn non_empty_owned(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed.to_owned())
}
