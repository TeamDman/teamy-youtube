use crate::cli::output::CliOutput;
use crate::cli::takeout::inspect::TakeoutInspectArgs;
use crate::cli::takeout::read::TakeoutReadArgs;
use crate::takeout_zip::ArchiveReadLimits;
use crate::takeout_zip::ArchiveSelectionRequest;
use arbitrary::Arbitrary;
use facet::Facet;
use figue as args;
use std::path::PathBuf;

/// Inspect or read Google Takeout ZIPs without extraction, import, or API requests.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct TakeoutArgs {
    #[facet(args::subcommand)]
    pub command: TakeoutCommand,
}

#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[repr(u8)]
pub enum TakeoutCommand {
    /// Select a ZIP and inspect its entry inventory without decoding watch records.
    Inspect(TakeoutInspectArgs),
    /// Decode normalized watch records from one bounded ZIP entry.
    Read(TakeoutReadArgs),
}

/// Shared selectors and limits for read-only ZIP operations.
#[derive(Facet, Arbitrary, Debug, Default, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct TakeoutZipArgs {
    /// Explicit ZIP path. Omit for automatic selection.
    #[facet(args::positional)]
    pub zip: Option<String>,
    /// Recursively discover ZIPs in this directory; repeat for multiple roots.
    /// Otherwise `TEAMY_YOUTUBE_TAKEOUT_DIR` or published MFT is used.
    #[facet(args::named, default)]
    pub input_dir: Vec<String>,
    /// Maximum decoded history entry bytes (default 268435456).
    #[facet(args::named)]
    pub max_entry_bytes: Option<u64>,
    /// Maximum ZIP entry count (default 100000).
    #[facet(args::named)]
    pub max_entries: Option<usize>,
    /// Maximum published MFT snapshot age, e.g. 2h (default 2h).
    #[facet(args::named)]
    pub mft_max_age: Option<String>,
}

impl TakeoutZipArgs {
    /// Prepare typed requests without opening an archive or acquiring credentials.
    /// # Errors
    /// Returns an error for invalid limits or discovery age.
    pub fn prepare(&self) -> eyre::Result<(ArchiveSelectionRequest, ArchiveReadLimits)> {
        let input_dirs = if !self.input_dir.is_empty() {
            self.input_dir.iter().map(PathBuf::from).collect()
        } else if self.zip.is_none() {
            std::env::var_os("TEAMY_YOUTUBE_TAKEOUT_DIR")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
                .into_iter()
                .collect()
        } else {
            Vec::new()
        };
        let mut limits = ArchiveReadLimits::default();
        if let Some(value) = self.max_entry_bytes {
            limits.max_entry_bytes = value;
        }
        if let Some(value) = self.max_entries {
            limits.max_entries = value;
        }
        eyre::ensure!(
            limits.max_entries > 0
                && limits.max_entry_bytes > 0
                && limits.max_entry_bytes < u64::MAX,
            "ZIP limits must be positive and byte limit must be below u64::MAX"
        );
        let age = humantime::parse_duration(self.mft_max_age.as_deref().unwrap_or("2h"))?;
        Ok((
            ArchiveSelectionRequest {
                zip: self.zip.as_ref().map(PathBuf::from),
                input_dirs,
                mft_max_age_seconds: age.as_secs(),
            },
            limits,
        ))
    }
}

impl TakeoutArgs {
    /// # Errors
    /// Returns an error if selection, inspection, or decoding fails.
    pub fn invoke(self) -> eyre::Result<CliOutput> {
        match self.command {
            TakeoutCommand::Inspect(args) => args.invoke(),
            TakeoutCommand::Read(args) => args.invoke(),
        }
    }
}
