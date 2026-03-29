use facet::Facet;

/// Raw watch-history subtitle block from Google Takeout.
#[derive(Clone, Debug, Facet, PartialEq)]
pub struct RawWatchHistorySubtitle {
    pub name: String,
    pub url: Option<String>,
}
