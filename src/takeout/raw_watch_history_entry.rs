use crate::takeout::raw_watch_history_subtitle::RawWatchHistorySubtitle;
use facet::Facet;

/// Raw entry shape from Google Takeout watch-history JSON.
#[derive(Debug, Facet, PartialEq)]
#[facet(derive(Default))]
pub struct RawWatchHistoryEntry {
    #[facet(default)]
    pub header: Option<String>,
    pub title: String,
    #[facet(rename = "titleUrl")]
    #[facet(default)]
    pub title_url: Option<String>,
    #[facet(default)]
    pub subtitles: Vec<RawWatchHistorySubtitle>,
    #[facet(rename = "time")]
    pub time: String,
}
